use super::{column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys};
use crate::{
    block::{store::BlockStore, BlockHash},
    canonicity::store::CanonicityStore,
    constants::*,
    event::{db::*, store::EventStore, IndexerEvent},
    ledger::{
        account::Account,
        diff::LedgerDiff,
        public_key::PublicKey,
        store::{
            best::BestLedgerStore,
            staged::{
                split_staged_account_balance_sort_key, staged_account_balance_sort_key,
                staged_account_key, StagedLedgerStore,
            },
        },
        Ledger, LedgerHash,
    },
    store::{from_be_bytes, IndexerStore},
};
use anyhow::{bail, Context};
use log::{error, trace};
use speedb::{DBIterator, Direction, IteratorMode, WriteBatch};
use std::collections::HashMap;

impl StagedLedgerStore for IndexerStore {
    fn get_staged_account(
        &self,
        pk: PublicKey,
        state_hash: BlockHash,
    ) -> anyhow::Result<Option<Account>> {
        trace!("Getting {pk} staged ledger {state_hash} account");

        // check if the account is in a staged ledger
        match self.get_pk_min_staged_ledger_block(&pk)? {
            None => {
                // account is not preset in a staged ledger
                return Ok(None);
            }
            Some(pk_min_block_height) => {
                if let Some(block_height) = self.get_block_height(&state_hash)? {
                    if pk_min_block_height > block_height {
                        return Ok(None);
                    }
                }
            }
        }

        // calculate account from canonical ancestor if needed
        let mut apply_block_diffs = vec![];
        let mut curr_state_hash = state_hash;

        while self
            .database
            .get_cf(
                self.staged_ledger_accounts_cf(),
                staged_account_key(curr_state_hash.clone(), pk.clone()),
            )?
            .is_none()
        {
            if let Some(parent_hash) = self.get_block_parent_hash(&curr_state_hash)? {
                apply_block_diffs.push(curr_state_hash.clone());
                curr_state_hash = parent_hash;
            } else {
                bail!("Block {curr_state_hash} missing parent from store")
            }
        }

        apply_block_diffs.reverse();

        let mut staged_account = self
            .database
            .get_cf(
                self.staged_ledger_accounts_cf(),
                staged_account_key(curr_state_hash.clone(), pk.clone()),
            )?
            .and_then(|bytes| serde_json::from_slice::<Account>(&bytes).ok())
            .with_context(|| format!("pk {pk}, state hash {curr_state_hash}"))
            .expect("account exists");

        for diff in apply_block_diffs
            .iter()
            .flat_map(|state_hash| self.get_block_ledger_diff(state_hash).ok().flatten())
        {
            staged_account = staged_account.apply_ledger_diff(&diff);
        }
        Ok(Some(staged_account))
    }

    fn get_staged_account_display(
        &self,
        pk: PublicKey,
        state_hash: BlockHash,
    ) -> anyhow::Result<Option<Account>> {
        trace!("Display {pk} staged ledger {state_hash} account");
        if let Some(staged_acct) = self.get_staged_account(pk, state_hash)? {
            return Ok(Some(staged_acct.display()));
        }
        Ok(None)
    }

    fn get_staged_account_block_height(
        &self,
        pk: PublicKey,
        block_height: u32,
    ) -> anyhow::Result<Option<Account>> {
        trace!("Getting {pk} staged ledger account block height {block_height}");
        let state_hash =
            if let Some(state_hash) = self.get_canonical_hash_at_height(block_height)? {
                state_hash
            } else {
                bail!("Missing canonical block at height {block_height}")
            };
        self.get_staged_account(pk, state_hash)
    }

    fn set_staged_account(
        &self,
        pk: PublicKey,
        state_hash: BlockHash,
        account: &Account,
    ) -> anyhow::Result<()> {
        let block_height = match self.get_block_height(&state_hash)? {
            None => bail!("Block missing from store {state_hash}"),
            Some(block_height) => block_height,
        };
        let block_height = self
            .get_pk_min_staged_ledger_block(&pk)?
            .map_or(block_height, |pk_min_block_height| {
                block_height.min(pk_min_block_height)
            });

        self.set_pk_min_staged_ledger_block(&pk, block_height)?;
        self.database.put_cf(
            self.staged_ledger_accounts_cf(),
            staged_account_key(state_hash.clone(), pk.clone()),
            serde_json::to_vec(&account)?,
        )?;
        self.database.put_cf(
            self.staged_ledger_account_balance_sort_cf(),
            staged_account_balance_sort_key(state_hash, account.balance.0, pk),
            serde_json::to_vec(&account)?,
        )?;
        Ok(())
    }

    fn get_pk_min_staged_ledger_block(&self, pk: &PublicKey) -> anyhow::Result<Option<u32>> {
        trace!("Getting pk min staged ledger block height {pk}");
        Ok(self
            .database
            .get_cf(self.staged_ledger_accounts_min_block_cf(), pk.0.as_bytes())?
            .map(from_be_bytes))
    }

    fn set_pk_min_staged_ledger_block(
        &self,
        pk: &PublicKey,
        block_height: u32,
    ) -> anyhow::Result<()> {
        trace!("Setting pk {pk} min staged ledger block height {block_height}");
        Ok(self.database.put_cf(
            self.staged_ledger_accounts_min_block_cf(),
            pk.0.as_bytes(),
            block_height.to_be_bytes(),
        )?)
    }

    fn add_staged_ledger_hashes(
        &self,
        ledger_hash: &LedgerHash,
        state_hash: &BlockHash,
    ) -> anyhow::Result<bool> {
        trace!("Adding staged ledger hash\nstate_hash: {state_hash}\nledger_hash: {ledger_hash}");
        let is_new = self
            .database
            .get_cf(self.staged_ledgers_persisted_cf(), state_hash.0.as_bytes())?
            .is_none();

        // record persistence
        self.database.put_cf(
            self.staged_ledgers_persisted_cf(),
            state_hash.0.as_bytes(),
            b"",
        )?;
        Ok(is_new)
    }

    fn add_staged_ledger_at_state_hash(
        &self,
        state_hash: &BlockHash,
        ledger: Ledger,
    ) -> anyhow::Result<()> {
        trace!("Adding staged ledger at state hash {state_hash}");

        // add staged accounts
        for (pk, account) in ledger.accounts.iter() {
            self.set_staged_account(pk.clone(), state_hash.clone(), account)?;
        }

        // index on state hash & add new ledger event
        if self
            .get_known_genesis_prev_state_hashes()?
            .contains(state_hash)
        {
            if self
                .add_staged_ledger_hashes(
                    &LedgerHash(MAINNET_GENESIS_LEDGER_HASH.into()),
                    state_hash,
                )
                .unwrap_or(false)
            {
                self.add_event(&IndexerEvent::Db(DbEvent::Ledger(
                    DbLedgerEvent::NewLedger {
                        blockchain_length: 0,
                        state_hash: state_hash.clone(),
                        ledger_hash: LedgerHash(MAINNET_GENESIS_LEDGER_HASH.into()),
                    },
                )))?;
            }
        } else {
            match self.get_block_staged_ledger_hash(state_hash)? {
                Some(ledger_hash) => {
                    if self
                        .add_staged_ledger_hashes(&ledger_hash, state_hash)
                        .unwrap_or(false)
                    {
                        self.add_event(&IndexerEvent::Db(DbEvent::Ledger(
                            DbLedgerEvent::NewLedger {
                                ledger_hash,
                                state_hash: state_hash.clone(),
                                blockchain_length: self
                                    .get_block_height(state_hash)?
                                    .expect("block height exists"),
                            },
                        )))?;
                    }
                }
                None => {
                    if state_hash.0 != MAINNET_GENESIS_PREV_STATE_HASH {
                        bail!("Block missing from store: {state_hash}")
                    }
                }
            }
        }

        // record persistence
        self.database.put_cf(
            self.staged_ledgers_persisted_cf(),
            state_hash.0.as_bytes(),
            b"",
        )?;
        Ok(())
    }

    fn add_genesis_ledger(
        &self,
        state_hash: &BlockHash,
        genesis_ledger: Ledger,
    ) -> anyhow::Result<()> {
        // add prev genesis state hash
        let mut known_prev = self.get_known_genesis_prev_state_hashes()?;
        if !known_prev.contains(state_hash) {
            known_prev.push(state_hash.clone());
            self.database.put(
                Self::KNOWN_GENESIS_PREV_STATE_HASHES_KEY,
                serde_json::to_vec(&known_prev)?,
            )?;
        }

        // initialize account balances for best ledger & sorting
        for (pk, acct) in genesis_ledger.accounts.iter() {
            self.update_best_account(pk, Some(acct.clone()))?;
        }
        self.add_staged_ledger_at_state_hash(
            state_hash,
            Ledger {
                accounts: genesis_ledger.accounts,
            },
        )?;
        Ok(())
    }

    fn get_staged_ledger_at_state_hash(
        &self,
        state_hash: &BlockHash,
        memoize: bool,
    ) -> anyhow::Result<Option<Ledger>> {
        trace!("Getting staged ledger state hash {state_hash}");
        let mut curr_state_hash = state_hash.clone();
        let mut diffs = vec![];

        // walk chain back to a stored ledger
        // collect diffs to compute the current ledger
        while self
            .database
            .get_cf(
                self.staged_ledgers_persisted_cf(),
                curr_state_hash.0.as_bytes(),
            )?
            .is_none()
        {
            trace!("No staged ledger found for state hash {curr_state_hash}");
            if let Some(diff) = self.get_block_ledger_diff(&curr_state_hash)? {
                diffs.push(diff);
                if let Ok(Some(parent_hash)) = self.get_block_parent_hash(&curr_state_hash) {
                    trace!("Checking for staged ledger state hash {parent_hash}");
                    curr_state_hash = parent_hash;
                }
            } else {
                if curr_state_hash.0 != MAINNET_GENESIS_PREV_STATE_HASH {
                    error!("Block missing from store: {curr_state_hash}");
                }
                return Ok(None);
            }
        }

        trace!("Found staged ledger state hash {curr_state_hash}");
        if let Ok(Some(mut ledger)) = self.build_staged_ledger(&curr_state_hash) {
            // apply diffs
            diffs.reverse();
            let diff = LedgerDiff::append_vec(diffs);
            ledger._apply_diff(&diff)?;

            if memoize {
                trace!("Memoizing ledger for block {state_hash}");
                self.add_staged_ledger_at_state_hash(state_hash, ledger.clone())?;
            }
            return Ok(Some(ledger));
        }
        Ok(None)
    }

    fn get_staged_ledger_at_ledger_hash(
        &self,
        ledger_hash: &LedgerHash,
        memoize: bool,
    ) -> anyhow::Result<Option<Ledger>> {
        trace!("Getting staged ledger hash {ledger_hash}");
        let key = ledger_hash.0.as_bytes();
        if let Some(state_hash) = self
            .database
            .get_cf(self.staged_ledger_hash_to_block_cf(), key)?
            .and_then(|bytes| BlockHash::from_bytes(&bytes).ok())
        {
            if let Some(ledger) = self.get_staged_ledger_at_state_hash(&state_hash, memoize)? {
                return Ok(Some(ledger));
            }
        }
        Ok(None)
    }

    fn get_staged_ledger_at_block_height(
        &self,
        height: u32,
        memoize: bool,
    ) -> anyhow::Result<Option<Ledger>> {
        trace!("Getting staged ledger at height {height}");
        self.get_canonical_hash_at_height(height)?
            .map_or(Ok(None), |state_hash| {
                self.get_staged_ledger_at_state_hash(&state_hash, memoize)
            })
    }

    fn set_block_ledger_diff_batch(
        &self,
        state_hash: &BlockHash,
        ledger_diff: LedgerDiff,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        trace!("Setting block ledger diff {state_hash}: {ledger_diff:?}");
        batch.put_cf(
            self.block_ledger_diff_cf(),
            state_hash.0.as_bytes(),
            serde_json::to_vec(&ledger_diff)?,
        );
        Ok(())
    }

    fn set_block_staged_ledger_hash_batch(
        &self,
        state_hash: &BlockHash,
        staged_ledger_hash: &LedgerHash,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        trace!("Setting block staged ledger hash {state_hash}: {staged_ledger_hash}");
        batch.put_cf(
            self.staged_ledger_hash_to_block_cf(),
            staged_ledger_hash.0.as_bytes(),
            state_hash.0.as_bytes(),
        );
        batch.put_cf(
            self.block_staged_ledger_hash_cf(),
            state_hash.0.as_bytes(),
            staged_ledger_hash.0.as_bytes(),
        );
        Ok(())
    }

    fn get_block_staged_ledger_hash(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<LedgerHash>> {
        trace!("Getting block staged ledger hash {state_hash}");
        Ok(self
            .database
            .get_cf(self.block_staged_ledger_hash_cf(), state_hash.0.as_bytes())?
            .and_then(|bytes| LedgerHash::from_bytes(bytes).ok()))
    }

    fn get_staged_ledger_block_state_hash(
        &self,
        ledger_hash: &LedgerHash,
    ) -> anyhow::Result<Option<BlockHash>> {
        trace!("Getting staged ledger {ledger_hash} block state hash");
        Ok(self
            .database
            .get_cf(
                self.staged_ledger_hash_to_block_cf(),
                ledger_hash.0.as_bytes(),
            )?
            .and_then(|bytes| BlockHash::from_bytes(&bytes).ok()))
    }

    fn build_staged_ledger(&self, state_hash: &BlockHash) -> anyhow::Result<Option<Ledger>> {
        trace!("Building staged ledger {state_hash}");
        let mut accounts = HashMap::new();
        for (key, value) in self
            .staged_ledger_account_balance_iterator(state_hash, Direction::Reverse)
            .flatten()
        {
            if let Some((key_state_hash, _, pk)) = split_staged_account_balance_sort_key(&key) {
                if key_state_hash != *state_hash {
                    // we've gone beyond the desired ledger accounts
                    break;
                }
                accounts.insert(pk, serde_json::from_slice(&value)?);
            } else {
                panic!("Invalid staged ledger account balance sort key");
            }
        }
        Ok(Some(Ledger { accounts }))
    }

    ///////////////
    // Iterators //
    ///////////////

    fn staged_ledger_account_balance_iterator(
        &self,
        state_hash: &BlockHash,
        direction: Direction,
    ) -> DBIterator<'_> {
        let mut start = state_hash.clone().to_bytes().to_vec();
        let mode = IteratorMode::From(
            match direction {
                Direction::Forward => start.as_slice(),
                Direction::Reverse => {
                    // need to "overshoot" all {state_hash}{pk} keys for this staged ledger
                    // without going into the "next" staged ledger's data
                    start.append(&mut "C".as_bytes().to_vec());
                    start.as_slice()
                }
            },
            direction,
        );
        self.database
            .iterator_cf(self.staged_ledger_account_balance_sort_cf(), mode)
    }
}
