use serde::{Deserialize, Serialize};

pub mod block;
pub mod db;
pub mod ledger;
pub mod store;
pub mod witness_tree;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum IndexerEvent {
    Db(db::DbEvent),
    BlockWatcher(block::BlockWatcherEvent),
    StakingLedgerWatcher(ledger::StakingLedgerWatcherEvent),
    WitnessTree(witness_tree::WitnessTreeEvent),
}

impl IndexerEvent {
    pub fn is_canonical_block_event(&self) -> bool {
        matches!(
            self,
            Self::Db(db::DbEvent::Canonicity(
                db::DbCanonicityEvent::NewCanonicalBlock { .. }
            ))
        )
    }

    pub fn is_new_block_event(&self) -> bool {
        matches!(
            self,
            Self::Db(db::DbEvent::Block(db::DbBlockEvent::NewBlock { .. }))
        )
    }
}

impl std::fmt::Debug for IndexerEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BlockWatcher(block_event) => write!(f, "{:?}", block_event),
            Self::Db(db_event) => write!(f, "{:?}", db_event),
            Self::StakingLedgerWatcher(ledger_event) => write!(f, "{:?}", ledger_event),
            Self::WitnessTree(tree_event) => write!(f, "{:?}", tree_event),
        }
    }
}
