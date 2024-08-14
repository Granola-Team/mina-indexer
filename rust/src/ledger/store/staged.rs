//! Store of staged ledgers

use crate::{
    block::BlockHash,
    ledger::{diff::LedgerDiff, Ledger, LedgerHash},
};

pub trait StagedLedgerStore {
    /// Add a ledger with assoociated hashes
    /// Returns true if ledger already present
    fn add_ledger(&self, ledger_hash: &LedgerHash, state_hash: &BlockHash) -> anyhow::Result<bool>;

    /// Add a ledger associated with a canonical block
    fn add_ledger_state_hash(&self, state_hash: &BlockHash, ledger: Ledger) -> anyhow::Result<()>;

    /// Add a new genesis ledger
    fn add_genesis_ledger(
        &self,
        state_hash: &BlockHash,
        genesis_ledger: Ledger,
    ) -> anyhow::Result<()>;

    /// Get a ledger associated with ledger hash
    fn get_ledger(&self, ledger_hash: &LedgerHash) -> anyhow::Result<Option<Ledger>>;

    /// Get a ledger associated with an arbitrary block
    fn get_ledger_state_hash(
        &self,
        state_hash: &BlockHash,
        memoize: bool,
    ) -> anyhow::Result<Option<Ledger>>;

    /// Get a (canonical) ledger at a specified block height
    /// (i.e. blockchain_length)
    fn get_ledger_block_height(&self, height: u32, memoize: bool)
        -> anyhow::Result<Option<Ledger>>;

    /// Index the block's ledger diff
    fn set_block_ledger_diff(
        &self,
        state_hash: &BlockHash,
        ledger_diff: LedgerDiff,
    ) -> anyhow::Result<()>;

    /// Index the block's ledger diff
    fn set_block_staged_ledger_hash(
        &self,
        state_hash: &BlockHash,
        staged_ledger_hash: &LedgerHash,
    ) -> anyhow::Result<()>;

    fn get_block_staged_ledger_hash(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<LedgerHash>>;
}
