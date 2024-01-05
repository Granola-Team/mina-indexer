use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DbEvent {
    Block(DbBlockEvent),
    Canonicity(DbCanonicityEvent),
    Ledger(DbLedgerEvent),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DbBlockEvent {
    AlreadySeenBlock {
        state_hash: String,
        blockchain_length: u32,
    },
    NewBlock {
        state_hash: String,
        blockchain_length: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DbLedgerEvent {
    AlreadySeenLedger(String),
    NewLedger { hash: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DbCanonicityEvent {
    NewCanonicalBlock {
        blockchain_length: u32,
        state_hash: String,
    },
}
