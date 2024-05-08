use crate::{block::BlockHash, chain_id::Network, ledger::LedgerHash};
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
        network: Network,
        state_hash: BlockHash,
        blockchain_length: u32,
    },
    NewBestTip {
        network: Network,
        state_hash: BlockHash,
        blockchain_length: u32,
    },
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum DbLedgerEvent {
    NewLedger {
        network: Network,
        state_hash: BlockHash,
        blockchain_length: u32,
        ledger_hash: LedgerHash,
    },
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum DbStakingLedgerEvent {
    NewStakingLedger {
        network: Network,
        epoch: u32,
        ledger_hash: LedgerHash,
    },
    AggregateDelegations {
        network: Network,
        epoch: u32,
    },
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum DbCanonicityEvent {
    NewCanonicalBlock {
        network: Network,
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
                network,
                state_hash,
                blockchain_length,
            } => write!(
                f,
                "db new {} block (length {}): {}",
                network, blockchain_length, state_hash
            ),
            Self::NewBestTip {
                network,
                state_hash,
                blockchain_length,
            } => {
                write!(
                    f,
                    "db new {} best tip (length {}): {}",
                    network, blockchain_length, state_hash
                )
            }
        }
    }
}

impl std::fmt::Debug for DbCanonicityEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewCanonicalBlock {
                network,
                state_hash,
                blockchain_length,
            } => write!(
                f,
                "db new {} canonical block (length {}): {}",
                network, blockchain_length, state_hash
            ),
        }
    }
}

impl std::fmt::Debug for DbLedgerEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewLedger {
                network,
                state_hash,
                blockchain_length,
                ledger_hash,
            } => write!(
                f,
                "db new {} ledger {} for (length {}): {}",
                network, ledger_hash.0, blockchain_length, state_hash.0
            ),
        }
    }
}

impl std::fmt::Debug for DbStakingLedgerEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewStakingLedger {
                epoch, ledger_hash, ..
            } => write!(
                f,
                "db new staking ledger (epoch {}): {}",
                epoch, ledger_hash.0
            ),
            Self::AggregateDelegations { network, epoch } => {
                write!(f, "db aggregated delegations {} epoch {}", network, epoch)
            }
        }
    }
}
