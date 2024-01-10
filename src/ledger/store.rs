use crate::{block::BlockHash, ledger::Ledger};

/// Store of canonical ledgers
pub trait LedgerStore {
    /// Add a ledger associated with a canonical block
    fn add_ledger(&self, state_hash: &BlockHash, ledger: Ledger) -> anyhow::Result<()>;

    /// Get a ledger associated with an arbitrary block
    fn get_ledger(&self, state_hash: &BlockHash) -> anyhow::Result<Option<Ledger>>;

    /// Get a ledger at a specified `blockchain_length`
    fn get_ledger_at_height(&self, height: u32) -> anyhow::Result<Option<Ledger>>;
}
