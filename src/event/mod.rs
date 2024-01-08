use serde::{Deserialize, Serialize};

pub mod block;
pub mod db;
pub mod ledger;
pub mod state;
pub mod store;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum Event {
    Block(block::BlockEvent),
    Db(db::DbEvent),
    Ledger(ledger::LedgerEvent),
    State(state::StateEvent),
}

impl Event {
    pub fn is_canonical_block_event(&self) -> bool {
        matches!(
            self,
            Event::Db(db::DbEvent::Canonicity(
                db::DbCanonicityEvent::NewCanonicalBlock { .. }
            ))
        )
    }

    pub fn is_new_block_event(&self) -> bool {
        matches!(
            self,
            Event::Db(db::DbEvent::Block(db::DbBlockEvent::NewBlock { .. }))
        )
    }
}

impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Block(block_event) => write!(f, "{:?}", block_event),
            Self::Db(db_event) => write!(f, "{:?}", db_event),
            Self::Ledger(ledger_event) => write!(f, "{:?}", ledger_event),
            Self::State(state_event) => write!(f, "{:?}", state_event),
        }
    }
}
