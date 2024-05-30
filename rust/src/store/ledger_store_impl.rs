use super::column_families::ColumnFamilyHelpers;
use crate::{
    block::{store::BlockStore, BlockHash},
    canonicity::store::CanonicityStore,
    constants::*,
    event::{db::*, store::EventStore, IndexerEvent},
    ledger::{
        public_key::PublicKey,
        staking::{AggregatedEpochStakeDelegations, StakingLedger},
        store::LedgerStore,
        Ledger, LedgerHash,
    },
    store::{account::AccountStore, IndexerStore},
};
use log::{error, trace};
use std::str::FromStr;

impl LedgerStore for IndexerStore {
    fn add_ledger(&self, ledger_hash: &LedgerHash, state_hash: &BlockHash) -> anyhow::Result<()> {
        trace!(
            "Adding staged ledger\nstate_hash: {}\nledger_hash: {}",
            state_hash.0,
            ledger_hash.0
        );

        // add state hash for ledger to db
        let key = ledger_hash.0.as_bytes();
        let value = state_hash.0.as_bytes();
        self.database.put_cf(self.ledgers_cf(), key, value)?;
        Ok(())
    }

    fn get_best_ledger(&self) -> anyhow::Result<Option<Ledger>> {
        trace!("Getting best ledger");
        self.get_ledger_state_hash(&self.get_best_block_hash()?.expect("best block"), true)
    }

    fn add_ledger_state_hash(&self, state_hash: &BlockHash, ledger: Ledger) -> anyhow::Result<()> {
        trace!("Adding staged ledger state hash {}", state_hash.0);

        // add ledger to db
        let key = state_hash.0.as_bytes();
        let value = ledger.to_string();
        let value = value.as_bytes();
        self.database.put_cf(self.ledgers_cf(), key, value)?;

        // index on state hash & add new ledger event
        if self.get_known_genesis_state_hashes()?.contains(state_hash) {
            self.add_ledger(&LedgerHash(MAINNET_GENESIS_LEDGER_HASH.into()), state_hash)?;
            self.add_event(&IndexerEvent::Db(DbEvent::Ledger(
                DbLedgerEvent::NewLedger {
                    blockchain_length: 0,
                    state_hash: state_hash.clone(),
                    ledger_hash: LedgerHash(MAINNET_GENESIS_LEDGER_HASH.into()),
                },
            )))?;
        } else {
            match self.get_block(state_hash)? {
                Some(block) => {
                    let ledger_hash = block.staged_ledger_hash();
                    self.add_ledger(&ledger_hash, state_hash)?;
                    self.add_event(&IndexerEvent::Db(DbEvent::Ledger(
                        DbLedgerEvent::NewLedger {
                            ledger_hash,
                            state_hash: block.state_hash(),
                            blockchain_length: block.blockchain_length(),
                        },
                    )))?;
                }
                None => error!("Block missing from store {}", state_hash.0),
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
            self.update_account_balance(pk, Some(acct.balance.0))?;
        }

        // add the ledger to the db
        self.add_ledger_state_hash(state_hash, genesis_ledger)?;
        Ok(())
    }

    fn get_ledger_state_hash(
        &self,
        state_hash: &BlockHash,
        memoize: bool,
    ) -> anyhow::Result<Option<Ledger>> {
        trace!("Getting staged ledger state hash {}", state_hash.0);

        let mut state_hash = state_hash.clone();
        let mut to_apply = vec![];

        // walk chain back to a stored ledger
        // collect blocks to compute the current ledger
        while self
            .database
            .get_pinned_cf(self.ledgers_cf(), state_hash.0.as_bytes())?
            .is_none()
        {
            trace!("No staged ledger found for state hash {}", state_hash);
            if let Some(block) = self.get_block(&state_hash)? {
                to_apply.push(block.clone());
                state_hash = block.previous_state_hash();
                trace!(
                    "Checking for staged ledger state hash {}",
                    block.previous_state_hash().0
                );
            } else {
                error!("Block missing from store: {}", state_hash.0);
                return Ok(None);
            }
        }

        trace!("Found staged ledger state hash {}", state_hash.0);
        to_apply.reverse();

        if let Some(mut ledger) = self
            .database
            .get_pinned_cf(self.ledgers_cf(), state_hash.0.as_bytes())?
            .map(|bytes| bytes.to_vec())
            .map(|bytes| Ledger::from_str(&String::from_utf8(bytes).unwrap()).unwrap())
        {
            if let Some(requested_block) = to_apply.last() {
                for block in &to_apply {
                    ledger._apply_diff_from_precomputed(block)?;
                }

                if memoize {
                    trace!("Memoizing ledger for block {}", requested_block.summary());
                    self.add_ledger_state_hash(&requested_block.state_hash(), ledger.clone())?;
                    self.add_event(&IndexerEvent::Db(DbEvent::Ledger(
                        DbLedgerEvent::NewLedger {
                            state_hash: requested_block.state_hash(),
                            ledger_hash: requested_block.staged_ledger_hash(),
                            blockchain_length: requested_block.blockchain_length(),
                        },
                    )))?;
                }
            }
            return Ok(Some(ledger));
        }
        Ok(None)
    }

    fn get_ledger(&self, ledger_hash: &LedgerHash) -> anyhow::Result<Option<Ledger>> {
        trace!("Getting staged ledger hash {}", ledger_hash.0);

        let key = ledger_hash.0.as_bytes();
        if let Some(state_hash) = self
            .database
            .get_pinned_cf(self.ledgers_cf(), key)?
            .map(|bytes| BlockHash(String::from_utf8(bytes.to_vec()).unwrap()))
        {
            let key = state_hash.0.as_bytes();
            if let Some(ledger) = self
                .database
                .get_pinned_cf(self.ledgers_cf(), key)?
                .map(|bytes| bytes.to_vec())
                .map(|bytes| Ledger::from_str(&String::from_utf8(bytes).unwrap()).unwrap())
            {
                return Ok(Some(ledger));
            }
        }
        Ok(None)
    }

    fn get_ledger_at_height(&self, height: u32, memoize: bool) -> anyhow::Result<Option<Ledger>> {
        trace!("Getting staged ledger height {}", height);

        match self.get_canonical_hash_at_height(height)? {
            None => Ok(None),
            Some(state_hash) => self.get_ledger_state_hash(&state_hash, memoize),
        }
    }

    fn get_staking_ledger_at_epoch(
        &self,
        epoch: u32,
        genesis_state_hash: &Option<BlockHash>,
    ) -> anyhow::Result<Option<StakingLedger>> {
        trace!("Getting staking ledger epoch {}", epoch);

        // default to current genesis state hash
        let genesis_state_hash = genesis_state_hash
            .clone()
            .unwrap_or(self.get_best_block()?.unwrap().genesis_state_hash());
        let key = format!("staking-{}-{}", genesis_state_hash.0, epoch);
        if let Some(ledger_result) = self
            .database
            .get_pinned_cf(self.ledgers_cf(), key.as_bytes())?
            .map(|bytes| bytes.to_vec())
            .map(|bytes| {
                let ledger_hash = String::from_utf8(bytes)?;
                self.get_staking_ledger_hash(&ledger_hash.into())
            })
        {
            return ledger_result;
        }
        Ok(None)
    }

    fn get_staking_ledger_hash(
        &self,
        ledger_hash: &LedgerHash,
    ) -> anyhow::Result<Option<StakingLedger>> {
        trace!("Getting staking ledger hash {}", ledger_hash.0);

        if let Some(bytes) = self
            .database
            .get_pinned_cf(self.ledgers_cf(), ledger_hash.0.as_bytes())?
        {
            return Ok(Some(serde_json::from_slice::<StakingLedger>(&bytes)?));
        }
        Ok(None)
    }

    fn add_staking_ledger(
        &self,
        staking_ledger: StakingLedger,
        genesis_state_hash: &BlockHash,
    ) -> anyhow::Result<()> {
        let epoch = staking_ledger.epoch;
        trace!("Adding staking ledger {}", staking_ledger.summary());

        // add ledger at ledger hash
        let key = staking_ledger.ledger_hash.0.as_bytes();
        let value = serde_json::to_vec(&staking_ledger)?;
        let is_new = self
            .database
            .get_pinned_cf(self.ledgers_cf(), key)?
            .is_none();
        self.database.put_cf(self.ledgers_cf(), key, value)?;

        // add (genesis state hash, epoch) index
        let key = format!("staking-{}-{}", genesis_state_hash.0, epoch);
        let value = staking_ledger.ledger_hash.0.as_bytes();
        self.database
            .put_cf(self.ledgers_cf(), key.as_bytes(), value)?;

        // aggregate staking delegations
        trace!("Aggregating staking delegations epoch {}", epoch);
        let aggregated_delegations = staking_ledger.aggregate_delegations()?;
        let key = format!("delegations-{}-{}", genesis_state_hash.0, epoch);
        self.database.put_cf(
            self.ledgers_cf(),
            key.as_bytes(),
            serde_json::to_vec(&aggregated_delegations)?,
        )?;

        if is_new {
            // add new ledger event
            self.add_event(&IndexerEvent::Db(DbEvent::StakingLedger(
                DbStakingLedgerEvent::NewStakingLedger {
                    epoch,
                    ledger_hash: staking_ledger.ledger_hash.clone(),
                    genesis_state_hash: genesis_state_hash.clone(),
                },
            )))?;

            // add new aggregated delegation event
            self.add_event(&IndexerEvent::Db(DbEvent::StakingLedger(
                DbStakingLedgerEvent::AggregateDelegations {
                    epoch: staking_ledger.epoch,
                    genesis_state_hash: genesis_state_hash.clone(),
                },
            )))?;
        }

        Ok(())
    }

    fn get_delegations_epoch(
        &self,
        epoch: u32,
        genesis_state_hash: &Option<BlockHash>,
    ) -> anyhow::Result<Option<AggregatedEpochStakeDelegations>> {
        trace!("Getting staking delegations for epoch {}", epoch);

        // default to current genesis state hash
        let genesis_state_hash = genesis_state_hash
            .clone()
            .unwrap_or(self.get_best_block()?.unwrap().genesis_state_hash());
        let key = format!("delegations-{}-{}", genesis_state_hash.0, epoch);

        if let Some(bytes) = self
            .database
            .get_pinned_cf(self.ledgers_cf(), key.as_bytes())?
        {
            return Ok(Some(serde_json::from_slice(&bytes)?));
        }
        Ok(None)
    }

    fn get_account_balance(&self, pk: &PublicKey) -> anyhow::Result<Option<u64>> {
        trace!("Getting account balance {pk}");

        Ok(self
            .database
            .get_cf(self.account_balance_cf(), pk.0.as_bytes())?
            .map(|bytes| {
                let mut be_bytes = [0; 8];
                be_bytes.copy_from_slice(&bytes[..8]);
                u64::from_be_bytes(be_bytes)
            }))
    }
}
