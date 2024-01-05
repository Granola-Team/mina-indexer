use serde::{Deserialize, Serialize};

pub mod block;
pub mod db;
pub mod ledger;
pub mod state;
pub mod store;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Event {
    Block(block::BlockEvent),
    Db(db::DbEvent),
    Ledger(ledger::LedgerEvent),
    State(state::StateEvent),
}
