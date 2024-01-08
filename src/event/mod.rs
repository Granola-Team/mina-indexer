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
