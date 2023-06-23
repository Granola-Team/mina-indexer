use crate::block::BlockHash;

use super::Ledger;

/// Store of canonical and epoch boundary ledgers
pub trait LedgerStore {
    /// Add ledger associated with a canonical block
    fn add_ledger(&self, state_hash: &BlockHash, ledger: Ledger) -> anyhow::Result<()>;

    /// Get a ledger associated with an arbitrary block
    fn get_ledger(&self, state_hash: &BlockHash) -> anyhow::Result<Option<Ledger>>;
}
