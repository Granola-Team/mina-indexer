//! Store of staking ledgers and delegations

use crate::{
    block::BlockHash,
    ledger::{
        staking::{AggregatedEpochStakeDelegations, StakingLedger},
        LedgerHash,
    },
};
use speedb::{DBIterator, IteratorMode};

pub trait StakingLedgerStore {
    /// Add a staking ledger
    fn add_staking_ledger(
        &self,
        staking_ledger: StakingLedger,
        genesis_state_hash: &BlockHash,
    ) -> anyhow::Result<()>;

    /// Get the staking ledger for the given epoch
    ///
    /// If no genesis state hash is provided, default to current network
    fn get_staking_ledger_at_epoch(
        &self,
        epoch: u32,
        genesis_state_hash: Option<BlockHash>,
    ) -> anyhow::Result<Option<StakingLedger>>;

    /// Get the staking ledger with the given hash
    fn get_staking_ledger_by_hash(
        &self,
        ledger_hash: &LedgerHash,
        epoch: Option<u32>,
        genesis_state_hash: Option<BlockHash>,
    ) -> anyhow::Result<Option<StakingLedger>>;

    /// Get the aggregated staking delegations for the given epoch
    ///
    /// If no genesis state hash is provided, default to current network
    fn get_delegations_epoch(
        &self,
        epoch: u32,
        genesis_state_hash: &Option<BlockHash>,
    ) -> anyhow::Result<Option<AggregatedEpochStakeDelegations>>;

    /// Set the epoch number corresponding to the given staking ledger hash
    fn set_ledger_hash_epoch_pair(
        &self,
        ledger_hash: &LedgerHash,
        epoch: u32,
    ) -> anyhow::Result<()>;

    /// Set the genesis state hash corresponding to the given staking ledger
    /// hash
    fn set_ledger_hash_genesis_pair(
        &self,
        ledger_hash: &LedgerHash,
        genesis_state_hash: &BlockHash,
    ) -> anyhow::Result<()>;

    /// Get the epoch number corresponding to the given staking ledger hash
    fn get_epoch(&self, ledger_hash: &LedgerHash) -> anyhow::Result<Option<u32>>;

    /// Get the staking ledger hash corresponding to the given epoch
    fn get_staking_ledger_hash_by_epoch(&self, epoch: u32) -> anyhow::Result<Option<LedgerHash>>;

    /// Get the genesis state hash corresponding to the given staking ledger
    fn get_genesis_state_hash(&self, ledger_hash: &LedgerHash)
        -> anyhow::Result<Option<BlockHash>>;

    /// Get the total number of accounts per staking ledger
    fn get_staking_ledger_accounts_count_epoch(
        &self,
        epoch: u32,
        genesis_state_hash: BlockHash,
    ) -> anyhow::Result<u32>;

    /// set the total number of accounts per staking ledger
    fn set_staking_ledger_accounts_count_epoch(
        &self,
        epoch: u32,
        genesis_state_hash: BlockHash,
        count: u32,
    ) -> anyhow::Result<()>;

    ///////////////
    // Iterators //
    ///////////////

    /// Per epoch staking ledger account iterator via balance
    fn staking_ledger_balance_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Per epoch staking ledger account iterator via stake (total delegations)
    fn staking_ledger_stake_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Per epoch staking ledger iterator via epoch
    fn staking_ledger_epoch_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;
}
