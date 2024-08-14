use super::column_families::ColumnFamilyHelpers;
use crate::{
    block::{store::BlockStore, BlockHash},
    canonicity::store::CanonicityStore,
    constants::*,
    event::{db::*, store::EventStore, IndexerEvent},
    ledger::{
        diff::LedgerDiff,
        store::{best::BestLedgerStore, staged::StagedLedgerStore},
        Ledger, LedgerHash,
    },
    store::IndexerStore,
};
use log::{error, trace};

impl StagedLedgerStore for IndexerStore {
    fn add_ledger(&self, ledger_hash: &LedgerHash, state_hash: &BlockHash) -> anyhow::Result<bool> {
        trace!("Adding staged ledger\nstate_hash: {state_hash}\nledger_hash: {ledger_hash}");
        let is_new = self
            .database
            .get_cf(self.ledgers_cf(), ledger_hash.0.as_bytes())?
            .is_none();
        self.database.put_cf(
            self.ledgers_cf(),
            ledger_hash.0.as_bytes(),
            state_hash.0.as_bytes(),
        )?;
        Ok(is_new)
    }

    fn add_ledger_state_hash(&self, state_hash: &BlockHash, ledger: Ledger) -> anyhow::Result<()> {
        trace!("Adding staged ledger state hash {state_hash}");

        // add ledger to db
        self.database.put_cf(
            self.ledgers_cf(),
            state_hash.0.as_bytes(),
            ledger.to_string(),
        )?;

        // index on state hash & add new ledger event
        if self
            .get_known_genesis_prev_state_hashes()?
            .contains(state_hash)
        {
            if self
                .add_ledger(&LedgerHash(MAINNET_GENESIS_LEDGER_HASH.into()), state_hash)
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
            match self.get_block(state_hash)? {
                Some((block, _)) => {
                    let ledger_hash = block.staged_ledger_hash();
                    if self.add_ledger(&ledger_hash, state_hash).unwrap_or(false) {
                        self.add_event(&IndexerEvent::Db(DbEvent::Ledger(
                            DbLedgerEvent::NewLedger {
                                ledger_hash,
                                state_hash: block.state_hash(),
                                blockchain_length: block.blockchain_length(),
                            },
                        )))?;
                    }
                }
                None => {
                    if state_hash.0 != MAINNET_GENESIS_PREV_STATE_HASH {
                        error!("Block missing from store: {state_hash}");
                    }
                }
            }
        }
        Ok(())
    }

    fn add_genesis_ledger(
        &self,
        state_hash: &BlockHash,
        genesis_ledger: Ledger,
    ) -> anyhow::Result<()> {
        // initialize account balances for sorting
        for (pk, acct) in &genesis_ledger.accounts {
            self.update_best_account(pk, Some(acct.clone()))?;
        }
        self.add_ledger_state_hash(state_hash, genesis_ledger)?;
        Ok(())
    }

    fn get_ledger_state_hash(
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
            .get_pinned_cf(self.ledgers_cf(), curr_state_hash.0.as_bytes())?
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
        if let Some(mut ledger) = self
            .database
            .get_pinned_cf(self.ledgers_cf(), curr_state_hash.0.as_bytes())?
            .and_then(|bytes| Ledger::from_bytes(bytes.to_vec()).ok())
        {
            // apply diffs
            diffs.reverse();
            let diff = LedgerDiff::append_vec(diffs);
            ledger._apply_diff(&diff)?;

            if memoize {
                trace!("Memoizing ledger for block {state_hash}");
                self.add_ledger_state_hash(state_hash, ledger.clone())?;
            }
            return Ok(Some(ledger));
        }
        Ok(None)
    }

    fn get_ledger(&self, ledger_hash: &LedgerHash) -> anyhow::Result<Option<Ledger>> {
        trace!("Getting staged ledger hash {ledger_hash}");
        let key = ledger_hash.0.as_bytes();
        if let Some(state_hash) = self
            .database
            .get_pinned_cf(self.ledgers_cf(), key)?
            .and_then(|bytes| BlockHash::from_bytes(&bytes).ok())
        {
            if let Some(ledger) = self
                .database
                .get_pinned_cf(self.ledgers_cf(), state_hash.0.as_bytes())?
                .and_then(|bytes| Ledger::from_bytes(bytes.to_vec()).ok())
            {
                return Ok(Some(ledger));
            }
        }
        Ok(None)
    }

    fn get_ledger_block_height(
        &self,
        height: u32,
        memoize: bool,
    ) -> anyhow::Result<Option<Ledger>> {
        trace!("Getting staged ledger at height {height}");
        self.get_canonical_hash_at_height(height)?
            .map_or(Ok(None), |state_hash| {
                self.get_ledger_state_hash(&state_hash, memoize)
            })
    }

    fn set_block_ledger_diff(
        &self,
        state_hash: &BlockHash,
        ledger_diff: LedgerDiff,
    ) -> anyhow::Result<()> {
        trace!("Setting block ledger diff {state_hash}: {ledger_diff:?}");
        Ok(self.database.put_cf(
            self.block_ledger_diff_cf(),
            state_hash.0.as_bytes(),
            serde_json::to_vec(&ledger_diff)?,
        )?)
    }

    fn set_block_staged_ledger_hash(
        &self,
        state_hash: &BlockHash,
        staged_ledger_hash: &LedgerHash,
    ) -> anyhow::Result<()> {
        trace!("Setting block staged ledger hash {state_hash}: {staged_ledger_hash}");
        Ok(self.database.put_cf(
            self.block_staged_ledger_hash_cf(),
            state_hash.0.as_bytes(),
            staged_ledger_hash.0.as_bytes(),
        )?)
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
}
