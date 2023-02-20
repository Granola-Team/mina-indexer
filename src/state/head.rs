use super::block::BlockHash;
use super::ledger::Ledger;

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Head {
    pub block_hash: BlockHash,
    pub ledger: Ledger,
}
