use crate::{block::BlockHash, chain::Network};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockWatcherEvent {
    SawBlock {
        network: Network,
        state_hash: BlockHash,
        blockchain_length: u32,
    },
}

impl std::fmt::Debug for BlockWatcherEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SawBlock {
                network,
                state_hash,
                blockchain_length,
            } => write!(
                f,
                "fs watcher saw {} block (length {}): {}",
                network, blockchain_length, state_hash.0
            ),
        }
    }
}
