use super::column_families::ColumnFamilyHelpers;
use crate::{
    block::{store::BlockStore, BlockHash},
    event::{db::*, store::EventStore, IndexerEvent},
    ledger::{
        public_key::PublicKey,
        staking::{AggregatedEpochStakeDelegations, StakingLedger},
        store::staking::StakingLedgerStore,
        LedgerHash,
    },
    store::{from_be_bytes, to_be_bytes, IndexerStore},
};
use log::trace;
use std::mem::size_of;

impl StakingLedgerStore for IndexerStore {
    fn get_staking_ledger_at_epoch(
        &self,
        epoch: u32,
        genesis_state_hash: Option<BlockHash>,
    ) -> anyhow::Result<Option<StakingLedger>> {
        trace!("Getting staking ledger epoch {epoch}");
        let genesis_state_hash = genesis_state_hash
            .clone()
            .unwrap_or_else(|| self.get_best_block_genesis_hash().ok().flatten().unwrap());
        if let Some(ledger_hash) = self.get_staking_ledger_hash_by_epoch(epoch)? {
            if let Some(ledger) = self
                .database
                .get_pinned_cf(
                    self.staking_ledgers_cf(),
                    staking_ledger_epoch_key(genesis_state_hash, epoch, &ledger_hash),
                )?
                .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            {
                return Ok(Some(ledger));
            }
        }
        Ok(None)
    }

    /// If some epoch is given, use it over the ledger hash,
    /// else get the epoch from the ledger hash
    fn get_staking_ledger_by_hash(
        &self,
        ledger_hash: &LedgerHash,
        epoch: Option<u32>,
        genesis_state_hash: Option<BlockHash>,
    ) -> anyhow::Result<Option<StakingLedger>> {
        trace!("Getting staking ledger hash {ledger_hash}");
        match epoch {
            None => {
                if let (Ok(Some(epoch)), Some(genesis_state_hash)) = (
                    self.get_epoch(ledger_hash),
                    genesis_state_hash
                        .or_else(|| self.get_best_block_genesis_hash().ok().flatten()),
                ) {
                    if let Ok(Some(bytes)) = self.database.get_pinned_cf(
                        self.staking_ledgers_cf(),
                        staking_ledger_epoch_key(genesis_state_hash, epoch, ledger_hash),
                    ) {
                        return Ok(Some(serde_json::from_slice(&bytes)?));
                    }
                }
                Ok(None)
            }
            Some(epoch) => {
                if let Ok(Some(staking_ledger)) =
                    self.get_staking_ledger_at_epoch(epoch, genesis_state_hash)
                {
                    if staking_ledger.ledger_hash == *ledger_hash {
                        return Ok(Some(staking_ledger));
                    }
                }
                Ok(None)
            }
        }
    }

    fn add_staking_ledger(
        &self,
        staking_ledger: StakingLedger,
        genesis_state_hash: &BlockHash,
    ) -> anyhow::Result<()> {
        let epoch = staking_ledger.epoch;
        trace!("Adding staking ledger {}", staking_ledger.summary());

        // add ledger at ledger hash
        let key = staking_ledger_epoch_key(
            genesis_state_hash.clone(),
            staking_ledger.epoch,
            &staking_ledger.ledger_hash,
        );
        let is_new = self
            .database
            .get_pinned_cf(self.staking_ledgers_cf(), key.clone())?
            .is_none();

        // add staking ledger
        self.database.put_cf(
            self.staking_ledgers_cf(),
            key,
            serde_json::to_vec(&staking_ledger)?,
        )?;

        // add (ledger hash, epoch) index
        self.set_ledger_hash_epoch_pair(&staking_ledger.ledger_hash, epoch)?;

        // add (ledger hash, genesis state hash) index
        self.set_ledger_hash_genesis_pair(&staking_ledger.ledger_hash, genesis_state_hash)?;

        // add aggregated delegations
        trace!("Aggregating staking delegations epoch {epoch}");
        let aggregated_delegations = staking_ledger.aggregate_delegations()?;
        self.database.put_cf(
            self.staking_delegations_cf(),
            staking_ledger_epoch_key_prefix(genesis_state_hash.clone(), epoch),
            serde_json::to_vec(&aggregated_delegations)?,
        )?;

        // add per epoch, balance-sorted & delegation-sorted
        for (pk, account) in staking_ledger.staking_ledger.iter() {
            // balance-sort
            self.database.put_cf(
                self.staking_ledger_balance_cf(),
                staking_ledger_sort_key(epoch, account.balance, &pk.0),
                serde_json::to_vec(account)?,
            )?;

            // stake-sort
            let stake = aggregated_delegations
                .delegations
                .get(pk)
                .cloned()
                .unwrap_or_default()
                .total_delegated
                .unwrap_or_default();
            self.database.put_cf(
                self.staking_ledger_stake_cf(),
                staking_ledger_sort_key(epoch, stake, &pk.0),
                serde_json::to_vec(account)?,
            )?;
        }
        // add staking ledger count at epoch
        let count = staking_ledger.staking_ledger.values().count();
        self.set_staking_ledger_accounts_count_epoch(
            epoch,
            genesis_state_hash.clone(),
            count as u32,
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
        trace!("Getting staking delegations for epoch {epoch}");
        let genesis_state_hash = genesis_state_hash
            .clone()
            .unwrap_or_else(|| self.get_best_block_genesis_hash().ok().flatten().unwrap());
        if let Some(bytes) = self.database.get_pinned_cf(
            self.staking_delegations_cf(),
            staking_ledger_epoch_key_prefix(genesis_state_hash, epoch),
        )? {
            return Ok(Some(serde_json::from_slice(&bytes)?));
        }
        Ok(None)
    }

    fn get_epoch(&self, ledger_hash: &LedgerHash) -> anyhow::Result<Option<u32>> {
        trace!("Getting epoch for ledger {ledger_hash}");
        Ok(self
            .database
            .get_cf(
                self.staking_ledger_hash_to_epoch_cf(),
                ledger_hash.0.as_bytes(),
            )?
            .map(from_be_bytes))
    }

    fn get_staking_ledger_hash_by_epoch(&self, epoch: u32) -> anyhow::Result<Option<LedgerHash>> {
        trace!("Getting ledger hash for epoch {epoch}");
        Ok(self
            .database
            .get_cf(self.staking_ledger_epoch_to_hash_cf(), to_be_bytes(epoch))?
            .and_then(|bytes| LedgerHash::from_bytes(bytes).ok()))
    }

    fn set_ledger_hash_epoch_pair(
        &self,
        ledger_hash: &LedgerHash,
        epoch: u32,
    ) -> anyhow::Result<()> {
        trace!("Setting epoch {epoch} for ledger {ledger_hash}");
        self.database.put_cf(
            self.staking_ledger_epoch_to_hash_cf(),
            to_be_bytes(epoch),
            ledger_hash.0.as_bytes(),
        )?;
        Ok(self.database.put_cf(
            self.staking_ledger_hash_to_epoch_cf(),
            ledger_hash.0.as_bytes(),
            to_be_bytes(epoch),
        )?)
    }

    fn set_ledger_hash_genesis_pair(
        &self,
        ledger_hash: &LedgerHash,
        genesis_state_hash: &BlockHash,
    ) -> anyhow::Result<()> {
        trace!("Setting genesis state hash {genesis_state_hash} for ledger {ledger_hash}");
        Ok(self.database.put_cf(
            self.staking_ledger_genesis_hash_cf(),
            ledger_hash.0.as_bytes(),
            genesis_state_hash.0.as_bytes(),
        )?)
    }

    fn get_genesis_state_hash(
        &self,
        ledger_hash: &LedgerHash,
    ) -> anyhow::Result<Option<BlockHash>> {
        trace!("Getting genesis state hash for ledger {ledger_hash}");
        Ok(self
            .database
            .get_cf(
                self.staking_ledger_genesis_hash_cf(),
                ledger_hash.0.as_bytes(),
            )?
            .and_then(|bytes| BlockHash::from_bytes(&bytes).ok()))
    }

    ///////////////
    // Iterators //
    ///////////////

    fn staking_ledger_balance_iterator(
        &self,
        mode: speedb::IteratorMode,
    ) -> speedb::DBIterator<'_> {
        self.database
            .iterator_cf(self.staking_ledger_balance_cf(), mode)
    }

    fn staking_ledger_stake_iterator(&self, mode: speedb::IteratorMode) -> speedb::DBIterator<'_> {
        self.database
            .iterator_cf(self.staking_ledger_stake_cf(), mode)
    }

    fn staking_ledger_epoch_iterator(&self, mode: speedb::IteratorMode) -> speedb::DBIterator<'_> {
        self.database.iterator_cf(self.staking_ledgers_cf(), mode)
    }

    ////////////////////////////
    // Staking ledger counts //
    ///////////////////////////

    fn get_staking_ledger_accounts_count_epoch(
        &self,
        epoch: u32,
        genesis_state_hash: BlockHash,
    ) -> anyhow::Result<u32> {
        trace!("Getting staking ledger accounts count for epoch {epoch} {genesis_state_hash:?}");
        Ok(self
            .database
            .get_cf(
                self.staking_ledger_accounts_epoch_cf(),
                staking_ledger_epoch_key_prefix(genesis_state_hash, epoch),
            )?
            .map_or(0, from_be_bytes))
    }

    fn set_staking_ledger_accounts_count_epoch(
        &self,
        epoch: u32,
        genesis_state_hash: BlockHash,
        count: u32,
    ) -> anyhow::Result<()> {
        trace!("Setting staking ledger accounts count for epoch {epoch} {genesis_state_hash:?}: {count}");
        Ok(self.database.put_cf(
            self.staking_ledger_accounts_epoch_cf(),
            staking_ledger_epoch_key_prefix(genesis_state_hash, epoch),
            to_be_bytes(count),
        )?)
    }
}

/// Staking ledger amount sort key
/// ```
/// {epoch BE}{amount BE}{suffix}
pub fn staking_ledger_sort_key(epoch: u32, amount: u64, suffix: &str) -> Vec<u8> {
    let mut key = to_be_bytes(epoch);
    key.append(&mut amount.to_be_bytes().to_vec());
    key.append(&mut suffix.as_bytes().to_vec());
    key
}

/// 4 BE bytes for epoch (u32)
pub fn staking_ledger_sort_key_epoch(key: &[u8]) -> u32 {
    from_be_bytes(key[..size_of::<u32>()].to_vec())
}

/// 8 BE bytes for amount (u64)
pub fn staking_ledger_sort_key_amount(key: &[u8]) -> u32 {
    from_be_bytes(key[size_of::<u32>()..][..size_of::<u64>()].to_vec())
}

/// Remaining bytes for public key
pub fn staking_ledger_sort_key_pk(key: &[u8]) -> PublicKey {
    let start_idx = size_of::<u32>() + size_of::<u64>();
    PublicKey::from_bytes(&key[start_idx..]).expect("public key from bytes")
}

/// Staking ledger epoch key
/// ```
/// {genesis state hash}{epoch BE}{ledger hash}
fn staking_ledger_epoch_key(
    genesis_state_hash: BlockHash,
    epoch: u32,
    ledger_hash: &LedgerHash,
) -> Vec<u8> {
    let mut key = staking_ledger_epoch_key_prefix(genesis_state_hash, epoch);
    key.append(&mut ledger_hash.0.clone().into_bytes());
    key
}

/// Prefix of [staking_ledger_epoch_key]
/// ```
/// {genesis state hash}{epoch BE}
fn staking_ledger_epoch_key_prefix(genesis_state_hash: BlockHash, epoch: u32) -> Vec<u8> {
    let mut key = genesis_state_hash.to_bytes();
    key.append(&mut to_be_bytes(epoch));
    key
}

/// Genesis state hash from [staking_ledger_epoch_key] bytes
pub fn staking_ledger_epoch_key_genesis_state_hash(key: &[u8]) -> BlockHash {
    BlockHash::from_bytes(&key[..BlockHash::LEN]).expect("genesis state hash from key")
}

/// Epoch from [staking_ledger_epoch_key] bytes
pub fn staking_ledger_epoch_key_epoch(key: &[u8]) -> u32 {
    from_be_bytes(key[BlockHash::LEN..][..size_of::<u32>()].to_vec())
}

/// Ledger hash from [staking_ledger_epoch_key] bytes
pub fn staking_ledger_epoch_key_ledger_hash(key: &[u8]) -> LedgerHash {
    LedgerHash::from_bytes(key[BlockHash::LEN + size_of::<u32>()..].to_vec())
        .expect("ledger hash from bytes")
}
