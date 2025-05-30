//! Staged ledger store impl

use super::{column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys, IndexerStore};
use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::store::BlockStore,
    canonicity::store::CanonicityStore,
    constants::*,
    event::{db::*, store::EventStore, IndexerEvent},
    ledger::{
        account::Account,
        diff::LedgerDiff,
        store::{
            best::BestLedgerStore,
            staged::{StagedLedgerStore, StateHashWithHeight},
        },
        token::{Token, TokenAddress, TokenSymbol},
        Ledger, LedgerHash,
    },
    store::{zkapp::tokens::ZkappTokenStore, Result},
    utility::store::ledger::staged::{
        split_staged_account_balance_sort_key, staged_account_balance_sort_key, staged_account_key,
    },
};
use anyhow::{bail, Context};
use log::{error, trace};
use speedb::{DBIterator, Direction, IteratorMode, WriteBatch};

impl StagedLedgerStore for IndexerStore {
    fn get_staged_account(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        state_hash: &StateHash,
    ) -> Result<Option<Account>> {
        trace!("Getting {} staged ledger {} account", pk, state_hash);

        // check if the account is in a sufficiently low staged ledger
        match self.get_pk_min_staged_ledger_block(pk)? {
            Some(pk_min_block) => {
                if let Some(block_height) = self.get_block_height(state_hash)? {
                    if pk_min_block.blockchain_length > block_height {
                        return Ok(None);
                    }
                }
            }
            None => return Ok(None),
        }

        // calculate account from canonical ancestor if needed
        let mut apply_block_diffs = vec![];
        let mut curr_state_hash = state_hash.clone();

        while self
            .database
            .get_cf(
                self.staged_ledger_accounts_cf(),
                staged_account_key(&curr_state_hash, token, pk),
            )?
            .is_none()
        {
            if let Some(parent_hash) = self.get_block_parent_hash(&curr_state_hash)? {
                apply_block_diffs.push(curr_state_hash.clone());
                curr_state_hash = parent_hash;
            } else {
                bail!("Block {} missing parent from store", curr_state_hash)
            }
        }

        apply_block_diffs.reverse();

        let mut staged_account = self
            .database
            .get_cf(
                self.staged_ledger_accounts_cf(),
                staged_account_key(&curr_state_hash, token, pk),
            )?
            .map(|bytes| serde_json::from_slice::<Account>(&bytes).expect("staged account"))
            .with_context(|| format!("pk {} state hash {}", pk, curr_state_hash))
            .expect("account exists");

        for diff in apply_block_diffs
            .iter()
            .flat_map(|state_hash| self.get_block_ledger_diff(state_hash).expect("ledger diff"))
        {
            staged_account = staged_account.apply_ledger_diff(&diff);
        }

        Ok(Some(staged_account))
    }

    fn get_staged_account_display(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        state_hash: &StateHash,
    ) -> Result<Option<Account>> {
        trace!("Display {} staged ledger {} account", pk, state_hash);

        if let Some(staged_acct) = self.get_staged_account(pk, token, state_hash)? {
            return Ok(Some(staged_acct.deduct_mina_account_creation_fee()));
        }

        Ok(None)
    }

    fn get_staged_account_block_height(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        block_height: u32,
    ) -> Result<Option<Account>> {
        trace!(
            "Getting {} staged ledger account block height {}",
            pk,
            block_height,
        );

        let state_hash =
            if let Some(state_hash) = self.get_canonical_hash_at_height(block_height)? {
                state_hash
            } else {
                bail!("Missing canonical block at height {}", block_height)
            };

        self.get_staged_account(pk, token, &state_hash)
    }

    fn set_staged_account(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        state_hash: &StateHash,
        block_height: u32,
        account: &Account,
    ) -> Result<()> {
        let account_serde_bytes = serde_json::to_vec(account)?;
        self.set_staged_account_raw_bytes(
            pk,
            token,
            state_hash,
            account.balance.0,
            block_height,
            &account_serde_bytes,
        )
    }

    fn set_staged_account_raw_bytes(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        state_hash: &StateHash,
        balance: u64,
        block_height: u32,
        account_serde_bytes: &[u8],
    ) -> Result<()> {
        trace!(
            "Setting staged account pk {} token {} height {} state hash {}",
            pk,
            token,
            block_height,
            state_hash,
        );

        // update pk min block height
        let pk_min_block = self.get_pk_min_staged_ledger_block(pk)?;
        if pk_min_block.is_none() || block_height < pk_min_block.unwrap().blockchain_length {
            self.set_pk_min_staged_ledger_block(
                pk,
                &StateHashWithHeight {
                    state_hash: state_hash.clone(),
                    blockchain_length: block_height,
                },
            )?;
        }

        // store staged ledger account bytes
        self.database.put_cf(
            self.staged_ledger_accounts_cf(),
            staged_account_key(state_hash, token, pk),
            account_serde_bytes,
        )?;

        // sort staged ledger account bytes
        self.database.put_cf(
            self.staged_ledger_account_balance_sort_cf(),
            staged_account_balance_sort_key(state_hash, token, balance, pk),
            account_serde_bytes,
        )?;

        Ok(())
    }

    fn remove_staged_account(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        state_hash: &StateHash,
        block_height: u32,
        balance: u64,
    ) -> Result<()> {
        trace!(
            "Removing staged account pk {} token {} height {} state hash {}",
            pk,
            token,
            block_height,
            state_hash,
        );

        // update pk min block height
        self.database
            .delete_cf(self.staged_ledger_accounts_min_block_cf(), pk.0.as_bytes())?;

        // store staged ledger account bytes
        self.database.delete_cf(
            self.staged_ledger_accounts_cf(),
            staged_account_key(state_hash, token, pk),
        )?;

        // sort staged ledger account bytes
        self.database.delete_cf(
            self.staged_ledger_account_balance_sort_cf(),
            staged_account_balance_sort_key(state_hash, token, balance, pk),
        )?;

        Ok(())
    }

    fn get_pk_min_staged_ledger_block(
        &self,
        pk: &PublicKey,
    ) -> Result<Option<StateHashWithHeight>> {
        trace!("Getting pk min staged ledger block height {}", pk);

        Ok(self
            .database
            .get_cf(self.staged_ledger_accounts_min_block_cf(), pk.0.as_bytes())?
            .map(|bytes| serde_json::from_slice(&bytes).expect("min staged block")))
    }

    fn set_pk_min_staged_ledger_block(
        &self,
        pk: &PublicKey,
        block_info: &StateHashWithHeight,
    ) -> Result<()> {
        trace!(
            "Setting min staged ledger block height {} hash {} pk {}",
            block_info.blockchain_length,
            block_info.state_hash,
            pk,
        );

        Ok(self.database.put_cf(
            self.staged_ledger_accounts_min_block_cf(),
            pk.0.as_bytes(),
            serde_json::to_vec(block_info)?,
        )?)
    }

    fn add_staged_ledger_hash(
        &self,
        ledger_hash: &LedgerHash,
        state_hash: &StateHash,
    ) -> Result<bool> {
        trace!(
            "Adding staged ledger hash\n  state_hash:  {}\n  ledger_hash: {}",
            state_hash,
            ledger_hash,
        );

        let is_new = self
            .database
            .get_cf(self.staged_ledgers_persisted_cf(), state_hash.0.as_bytes())?
            .is_none();

        // record persistence
        if is_new {
            self.database.put_cf(
                self.staged_ledgers_persisted_cf(),
                state_hash.0.as_bytes(),
                b"",
            )?;
        }

        Ok(is_new)
    }

    fn add_staged_ledger_at_state_hash(
        &self,
        state_hash: &StateHash,
        ledger: &Ledger,
        block_height: u32,
    ) -> Result<()> {
        trace!("Adding staged ledger at state hash {}", state_hash);

        // add staged accounts
        for (token, token_ledger) in ledger.tokens.iter() {
            for (pk, account) in token_ledger.accounts.iter() {
                self.set_staged_account(pk, token, state_hash, block_height, account)?;
            }
        }

        // index on state hash & add new ledger event
        if state_hash.0 == MAINNET_GENESIS_PREV_STATE_HASH
            && self
                .add_staged_ledger_hash(
                    &LedgerHash::new_or_panic(MAINNET_GENESIS_LEDGER_HASH.into()),
                    state_hash,
                )
                .unwrap_or(false)
        {
            self.add_event(&IndexerEvent::Db(DbEvent::Ledger(
                DbLedgerEvent::NewLedger {
                    blockchain_length: 0,
                    state_hash: state_hash.clone(),
                    ledger_hash: LedgerHash::new_or_panic(MAINNET_GENESIS_LEDGER_HASH.into()),
                },
            )))?;
        } else if state_hash.0 == HARDFORK_GENESIS_PREV_STATE_HASH
            && self
                .add_staged_ledger_hash(
                    &LedgerHash::new_or_panic(HARDFORK_GENESIS_LEDGER_HASH.into()),
                    state_hash,
                )
                .unwrap_or(false)
        {
            self.add_event(&IndexerEvent::Db(DbEvent::Ledger(
                DbLedgerEvent::NewLedger {
                    blockchain_length: HARDFORK_GENESIS_BLOCKCHAIN_LENGTH - 1,
                    state_hash: state_hash.clone(),
                    ledger_hash: LedgerHash::new_or_panic(HARDFORK_GENESIS_LEDGER_HASH.into()),
                },
            )))?;
        } else {
            match self.get_block_staged_ledger_hash(state_hash)? {
                Some(ledger_hash) => {
                    if self
                        .add_staged_ledger_hash(&ledger_hash, state_hash)
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
                    if !is_genesis_prev_state_hash(state_hash) {
                        bail!(
                            "Staged ledger hash block missing from store: {}",
                            state_hash,
                        )
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
        state_hash: &StateHash,
        genesis_ledger: &Ledger,
        height: u32,
        genesis_token: Option<&Token>,
    ) -> Result<()> {
        trace!("Adding genesis ledger {} to the store", state_hash);

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
        for (token, token_ledger) in genesis_ledger.tokens.iter() {
            for (pk, acct) in token_ledger.accounts.iter() {
                self.update_best_account(pk, token, None, Some(acct.clone()), true)?;
            }
        }

        // initialize account/zkapp/token counts
        let count = genesis_ledger.len() as u32;

        self.set_num_accounts(count)?;
        self.set_num_mina_accounts(count)?;
        self.set_mina_token_holders_num(count)?;
        self.set_num_zkapp_accounts(0)?;

        // initialize genesis token
        if let Some(token) = genesis_token {
            self.database.put_cf(
                self.zkapp_tokens_symbol_cf(),
                token.token.0.as_bytes(),
                serde_json::to_vec(&TokenSymbol::mina())?,
            )?;

            self.set_token(token)?;
        }

        self.add_staged_ledger_at_state_hash(state_hash, genesis_ledger, height)
    }

    fn get_staged_ledger_at_state_hash(
        &self,
        state_hash: &StateHash,
        memoize: bool,
    ) -> Result<Option<Ledger>> {
        trace!("Getting staged ledger state hash {}", state_hash);

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
                    trace!("Checking for staged ledger state hash {}", parent_hash);
                    curr_state_hash = parent_hash;
                }
            } else {
                if !is_genesis_prev_state_hash(&curr_state_hash) {
                    error!("Block missing from store: {}", curr_state_hash);
                }

                return Ok(None);
            }
        }

        trace!("Found staged ledger state hash {}", curr_state_hash);
        if let Ok(Some(mut ledger)) = self.build_staged_ledger(&curr_state_hash) {
            // apply diffs
            diffs.reverse();

            let diff = LedgerDiff::append_vec(diffs);
            ledger._apply_diff(&diff)?;

            if memoize {
                trace!("Memoizing ledger for block {}", state_hash);

                match self.get_block_height(state_hash)? {
                    Some(block_height) => {
                        self.add_staged_ledger_at_state_hash(state_hash, &ledger, block_height)?
                    }
                    None => bail!("Block missing from store {}", state_hash),
                }
            }

            return Ok(Some(ledger));
        }

        Ok(None)
    }

    fn get_staged_ledger_at_ledger_hash(
        &self,
        ledger_hash: &LedgerHash,
        memoize: bool,
    ) -> Result<Option<Ledger>> {
        trace!("Getting staged ledger hash {}", ledger_hash);

        let key = ledger_hash.0.as_bytes();
        if let Some(state_hash) = self
            .database
            .get_cf(self.staged_ledger_hash_to_block_cf(), key)?
            .map(|bytes| StateHash::from_bytes(&bytes).expect("state hash"))
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
    ) -> Result<Option<Ledger>> {
        trace!("Getting staged ledger at height {}", height);

        self.get_canonical_hash_at_height(height)?
            .map_or(Ok(None), |state_hash| {
                self.get_staged_ledger_at_state_hash(&state_hash, memoize)
            })
    }

    fn set_block_ledger_diff_batch(
        &self,
        state_hash: &StateHash,
        ledger_diff: &LedgerDiff,
        batch: &mut WriteBatch,
    ) -> Result<()> {
        trace!(
            "Setting block ledger diff {}: {:?}",
            state_hash,
            ledger_diff,
        );

        batch.put_cf(
            self.block_ledger_diff_cf(),
            state_hash.0.as_bytes(),
            serde_json::to_vec(ledger_diff)?,
        );
        Ok(())
    }

    fn set_block_staged_ledger_hash_batch(
        &self,
        state_hash: &StateHash,
        staged_ledger_hash: &LedgerHash,
        batch: &mut WriteBatch,
    ) -> Result<()> {
        trace!(
            "Setting block staged ledger hash {}: {}",
            state_hash,
            staged_ledger_hash,
        );

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

    fn get_block_staged_ledger_hash(&self, state_hash: &StateHash) -> Result<Option<LedgerHash>> {
        trace!("Getting block staged ledger hash {}", state_hash);

        Ok(self
            .database
            .get_cf(self.block_staged_ledger_hash_cf(), state_hash.0.as_bytes())?
            .map(|bytes| LedgerHash::from_bytes(bytes).expect("ledger hash")))
    }

    fn get_staged_ledger_block_state_hash(
        &self,
        ledger_hash: &LedgerHash,
    ) -> Result<Option<StateHash>> {
        trace!("Getting staged ledger {} block state hash", ledger_hash);

        Ok(self
            .database
            .get_cf(
                self.staged_ledger_hash_to_block_cf(),
                ledger_hash.0.as_bytes(),
            )?
            .map(StateHash::from_bytes_or_panic))
    }

    fn build_staged_ledger(&self, state_hash: &StateHash) -> Result<Option<Ledger>> {
        trace!("Building staged ledger {}", state_hash);

        let mut ledger = Ledger::new();
        for (key, value) in self
            .staged_ledger_account_balance_iterator(state_hash, Direction::Reverse)
            .flatten()
        {
            if let Some((key_state_hash, token, _, _)) = split_staged_account_balance_sort_key(&key)
            {
                if key_state_hash != *state_hash {
                    // we've gone beyond the desired ledger accounts
                    break;
                }

                let account = serde_json::from_slice(&value).expect("account serde bytes");
                ledger.insert_account(account, &token);
            } else {
                panic!("Invalid staged ledger account balance sort key");
            }
        }

        Ok(Some(ledger))
    }

    ///////////////
    // Iterators //
    ///////////////

    fn staged_ledger_account_balance_iterator(
        &self,
        state_hash: &StateHash,
        direction: Direction,
    ) -> DBIterator<'_> {
        let mut start = [0; StateHash::LEN + TokenAddress::LEN + 1];
        start[..StateHash::LEN].copy_from_slice(state_hash.0.as_bytes());

        if let Direction::Reverse = direction {
            // need to go beyond all {state_hash}{token}{pk} keys for this staged ledger
            // without going into the "next" staged ledger's data
            start[StateHash::LEN..][..TokenAddress::LEN]
                .copy_from_slice(&TokenAddress::upper_bound());
            start[StateHash::LEN..][TokenAddress::LEN..].copy_from_slice(b"C");
        }

        let mode = IteratorMode::From(&start, direction);
        self.database
            .iterator_cf(self.staged_ledger_account_balance_sort_cf(), mode)
    }
}

fn is_genesis_prev_state_hash(state_hash: &StateHash) -> bool {
    state_hash.0 == MAINNET_GENESIS_PREV_STATE_HASH
        || state_hash.0 == HARDFORK_GENESIS_PREV_STATE_HASH
}
