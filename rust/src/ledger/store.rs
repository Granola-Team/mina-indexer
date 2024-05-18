//! Store of staged ledgers, staking ledgers, and staking delegations

use crate::{
    block::BlockHash,
    ledger::{
        staking::{AggregatedEpochStakeDelegations, StakingLedger},
        Ledger, LedgerHash,
    },
};

pub trait LedgerStore {
    /// Add a ledger with assoociated hash
    fn add_ledger(&self, ledger_hash: &LedgerHash, state_hash: &BlockHash) -> anyhow::Result<()>;

    /// Add a ledger associated with a canonical block
    fn add_ledger_state_hash(&self, state_hash: &BlockHash, ledger: Ledger) -> anyhow::Result<()>;

    /// Get a ledger associated with ledger hash
    fn get_ledger(&self, ledger_hash: &LedgerHash) -> anyhow::Result<Option<Ledger>>;

    /// Get a ledger associated with an arbitrary block
    fn get_ledger_state_hash(
        &self,
        state_hash: &BlockHash,
        memoize: bool,
    ) -> anyhow::Result<Option<Ledger>>;

    /// Get a ledger at a specified `blockchain_length`
    fn get_ledger_at_height(&self, height: u32, memoize: bool) -> anyhow::Result<Option<Ledger>>;

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
        genesis_state_hash: &Option<BlockHash>,
    ) -> anyhow::Result<Option<StakingLedger>>;

    /// Get the staking ledger with the given hash
    fn get_staking_ledger_hash(&self, hash: &LedgerHash) -> anyhow::Result<Option<StakingLedger>>;

    /// Get the aggregated staking delegations for the given epoch
    ///
    /// If no genesis state hash is provided, default to current network
    fn get_delegations_epoch(
        &self,
        epoch: u32,
        genesis_state_hash: &Option<BlockHash>,
    ) -> anyhow::Result<Option<AggregatedEpochStakeDelegations>>;

    /// Persist ledger account balances
    fn store_ledger_balances(&self, ledger: &Ledger) -> anyhow::Result<()>;
}
