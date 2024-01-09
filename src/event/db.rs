use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum DbEvent {
    Block(DbBlockWatcherEvent),
    Canonicity(DbCanonicityEvent),
    Ledger(DbLedgerWatcherEvent),
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum DbBlockWatcherEvent {
    AlreadySeenBlock {
        state_hash: String,
        blockchain_length: u32,
    },
    NewBlock {
        state_hash: String,
        blockchain_length: u32,
    },
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum DbLedgerWatcherEvent {
    AlreadySeenLedger(String),
    NewLedger { hash: String },
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum DbCanonicityEvent {
    NewCanonicalBlock {
        blockchain_length: u32,
        state_hash: String,
    },
}

impl DbEvent {
    pub fn is_new_block_event(&self) -> bool {
        matches!(self, DbEvent::Block(DbBlockWatcherEvent::NewBlock { .. }))
    }
}

impl std::fmt::Debug for DbEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Block(db_block_event) => write!(f, "{:?}", db_block_event),
            Self::Canonicity(db_canonicity_event) => write!(f, "{:?}", db_canonicity_event),
            Self::Ledger(db_ledger_event) => write!(f, "{:?}", db_ledger_event),
        }
    }
}

impl std::fmt::Debug for DbBlockWatcherEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadySeenBlock {
                state_hash,
                blockchain_length,
            } => write!(
                f,
                "db: already seen block ({blockchain_length}, {state_hash})"
            ),
            Self::NewBlock {
                state_hash,
                blockchain_length,
                ..
            } => write!(f, "db: new block ({blockchain_length}, {state_hash})"),
        }
    }
}

impl std::fmt::Debug for DbCanonicityEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewCanonicalBlock {
                state_hash,
                blockchain_length,
            } => write!(
                f,
                "db: new canonical block ({blockchain_length}, {state_hash})"
            ),
        }
    }
}

impl std::fmt::Debug for DbLedgerWatcherEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadySeenLedger(hash) => write!(f, "db: already seen ledger ({hash})"),
            Self::NewLedger { hash, .. } => write!(f, "db: new ledger ({hash})"),
        }
    }
}
