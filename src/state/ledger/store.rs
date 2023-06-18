use crate::block::BlockHash;

use super::Ledger;

pub trait LedgerStore {
    fn add_ledger(&self, state_hash: &BlockHash, ledger: Ledger) -> anyhow::Result<()>;
    fn get_ledger(&self, state_hash: &BlockHash) -> anyhow::Result<Option<Ledger>>;
}
