//! Store of staking ledgers and delegations

use crate::{
    block::BlockHash,
    ledger::{
        public_key::PublicKey,
        staking::{
            AggregatedEpochStakeDelegations, EpochStakeDelegation, StakingAccount, StakingLedger,
        },
        LedgerHash,
    },
};
use speedb::{DBIterator, Direction, IteratorMode};

pub trait StakingLedgerStore {
    /// Get `pk`'s `epoch` staking ledger account
    fn get_staking_account(
        &self,
        pk: PublicKey,
        epoch: u32,
        genesis_state_hash: &Option<BlockHash>,
    ) -> anyhow::Result<Option<StakingAccount>>;

    /// Set `pk`'s staking ledger account
    fn set_staking_account(
        &self,
        pk: PublicKey,
        epoch: u32,
        ledger_hash: LedgerHash,
        genesis_state_hash: BlockHash,
        staking_account: &StakingAccount,
    ) -> anyhow::Result<()>;

    /// Add a staking ledger
    fn add_staking_ledger(
        &self,
        staking_ledger: StakingLedger,
        genesis_state_hash: &BlockHash,
    ) -> anyhow::Result<()>;

    /// Get the staking ledger with the given hash & epoch
    fn get_staking_ledger(
        &self,
        ledger_hash: &LedgerHash,
        epoch: Option<u32>,
        genesis_state_hash: &Option<BlockHash>,
    ) -> anyhow::Result<Option<StakingLedger>>;

    /// Get the aggregated staking delegations for the given epoch
    ///
    /// If no genesis state hash is provided, default to current network
    fn get_epoch_delegations(
        &self,
        pk: PublicKey,
        epoch: u32,
        genesis_state_hash: Option<BlockHash>,
    ) -> anyhow::Result<Option<EpochStakeDelegation>>;

    /// Set epoch stake delegations
    fn set_epoch_delegations(
        &self,
        pk: PublicKey,
        epoch: u32,
        ledger_hash: LedgerHash,
        genesis_state_hash: Option<BlockHash>,
        epoch_stake_delegation: &EpochStakeDelegation,
    ) -> anyhow::Result<()>;

    /// Set the epoch number corresponding to the given staking ledger hash
    fn set_staking_ledger_hash_epoch_pair(
        &self,
        ledger_hash: &LedgerHash,
        epoch: u32,
        genesis_state_hash: Option<BlockHash>,
    ) -> anyhow::Result<()>;

    /// Set the genesis state hash corresponding to the given staking ledger
    /// hash
    fn set_staking_ledger_hash_genesis_pair(
        &self,
        ledger_hash: &LedgerHash,
        genesis_state_hash: &BlockHash,
    ) -> anyhow::Result<()>;

    /// Get the epoch number corresponding to the given staking ledger hash
    fn get_epoch(&self, ledger_hash: &LedgerHash) -> anyhow::Result<Option<u32>>;

    /// Get the staking ledger hash corresponding to the given epoch
    fn get_staking_ledger_hash_by_epoch(
        &self,
        epoch: u32,
        genesis_state_hash: Option<BlockHash>,
    ) -> anyhow::Result<Option<LedgerHash>>;

    /// Get the genesis state hash corresponding to the given staking ledger
    fn get_genesis_state_hash(&self, ledger_hash: &LedgerHash)
        -> anyhow::Result<Option<BlockHash>>;

    /// Set a staking ledger's total currency
    fn set_total_currency(
        &self,
        ledger_hash: &LedgerHash,
        total_currency: u64,
    ) -> anyhow::Result<()>;

    /// Get a staking ledger's total currency
    fn get_total_currency(&self, ledger_hash: &LedgerHash) -> anyhow::Result<Option<u64>>;

    /// Get the total number of accounts per staking ledger
    fn get_staking_ledger_accounts_count_epoch(
        &self,
        epoch: u32,
        genesis_state_hash: BlockHash,
    ) -> anyhow::Result<u32>;

    /// Set the total number of accounts per staking ledger
    fn set_staking_ledger_accounts_count_epoch(
        &self,
        epoch: u32,
        genesis_state_hash: BlockHash,
        count: u32,
    ) -> anyhow::Result<()>;

    // Build the staking ledger from the CF representation
    fn build_staking_ledger(
        &self,
        epoch: u32,
        genesis_state_hash: &Option<BlockHash>,
    ) -> anyhow::Result<Option<StakingLedger>>;

    // Build the aggregated staking delegations from the CF representation
    fn build_aggregated_delegations(
        &self,
        epoch: u32,
        genesis_state_hash: &Option<BlockHash>,
    ) -> anyhow::Result<Option<AggregatedEpochStakeDelegations>>;

    ///////////////
    // Iterators //
    ///////////////

    /// Per epoch staking ledger account iterator via balance
    /// ```
    /// key: [staking_ledger_sort_key]
    /// val: b""
    fn staking_ledger_account_balance_iterator(
        &self,
        epoch: u32,
        direction: Direction,
    ) -> DBIterator<'_>;

    /// Per epoch staking ledger account iterator via stake (total delegations)
    /// ```
    /// key: [staking_ledger_sort_key]
    /// val: b""
    fn staking_ledger_account_stake_iterator(
        &self,
        epoch: u32,
        direction: Direction,
    ) -> DBIterator<'_>;

    /// Per epoch staking ledger iterator via epoch
    /// ```
    /// key: epoch BE bytes
    /// val: b""
    fn staking_ledger_epoch_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;
}
