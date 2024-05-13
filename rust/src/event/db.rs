use crate::{block::BlockHash, ledger::LedgerHash};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum DbEvent {
    Block(DbBlockEvent),
    Canonicity(DbCanonicityEvent),
    Ledger(DbLedgerEvent),
    StakingLedger(DbStakingLedgerEvent),
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum DbBlockEvent {
    NewBlock {
        state_hash: BlockHash,
        blockchain_length: u32,
    },
    NewBestTip {
        state_hash: BlockHash,
        blockchain_length: u32,
    },
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum DbLedgerEvent {
    NewLedger {
        state_hash: BlockHash,
        blockchain_length: u32,
        ledger_hash: LedgerHash,
    },
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum DbStakingLedgerEvent {
    NewStakingLedger {
        epoch: u32,
        ledger_hash: LedgerHash,
        genesis_state_hash: BlockHash,
    },
    AggregateDelegations {
        epoch: u32,
        genesis_state_hash: BlockHash,
    },
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum DbCanonicityEvent {
    NewCanonicalBlock {
        state_hash: BlockHash,
        blockchain_length: u32,
    },
}

impl DbEvent {
    pub fn is_new_block_event(&self) -> bool {
        matches!(self, DbEvent::Block(DbBlockEvent::NewBlock { .. }))
    }
}

impl std::fmt::Debug for DbEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Block(db_block_event) => write!(f, "{:?}", db_block_event),
            Self::Canonicity(db_canonicity_event) => write!(f, "{:?}", db_canonicity_event),
            Self::Ledger(db_ledger_event) => write!(f, "{:?}", db_ledger_event),
            Self::StakingLedger(db_ledger_event) => write!(f, "{:?}", db_ledger_event),
        }
    }
}

impl std::fmt::Debug for DbBlockEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewBlock {
                state_hash,
                blockchain_length,
            } => write!(
                f,
                "db new block (length {}): {}",
                blockchain_length, state_hash
            ),
            Self::NewBestTip {
                state_hash,
                blockchain_length,
            } => {
                write!(
                    f,
                    "db new best tip (length {}): {}",
                    blockchain_length, state_hash
                )
            }
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
                "db new canonical block (length {}): {}",
                blockchain_length, state_hash
            ),
        }
    }
}

impl std::fmt::Debug for DbLedgerEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewLedger {
                state_hash,
                blockchain_length,
                ledger_hash,
            } => write!(
                f,
                "db new ledger {} for (length {}): {}",
                ledger_hash.0, blockchain_length, state_hash.0
            ),
        }
    }
}

impl std::fmt::Debug for DbStakingLedgerEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewStakingLedger {
                epoch,
                ledger_hash,
                genesis_state_hash: _,
            } => write!(
                f,
                "db new staking ledger (epoch {}): {}",
                epoch, ledger_hash.0
            ),
            Self::AggregateDelegations {
                epoch,
                genesis_state_hash: _,
            } => {
                write!(f, "db aggregated delegations epoch {}", epoch)
            }
        }
    }
}
