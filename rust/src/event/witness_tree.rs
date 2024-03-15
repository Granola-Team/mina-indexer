//! State events
//!
//! State events are not recorded in the event log.
//! They are used to update the db.

use crate::block::Block;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum WitnessTreeEvent {
    UpdateBestTip(Block),
    UpdateCanonicalChain {
        best_tip: Block,
        canonical_blocks: CanonicalBlocksEvent,
    },
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum CanonicalBlocksEvent {
    CanonicalBlocks(Vec<Block>),
}

impl std::fmt::Debug for WitnessTreeEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UpdateBestTip(block) => write!(f, "{:?}", block),
            Self::UpdateCanonicalChain {
                best_tip,
                canonical_blocks,
            } => write!(f, "{:?}", (best_tip, canonical_blocks)),
        }
    }
}

impl std::fmt::Debug for CanonicalBlocksEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let CanonicalBlocksEvent::CanonicalBlocks(blocks) = self;
        write!(f, "{:?}", blocks)
    }
}
