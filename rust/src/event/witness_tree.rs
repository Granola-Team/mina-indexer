//! Witness tree events
//!
//! Witness tree events are not recorded in the event log.
//! They are only used to update the db.

use crate::block::Block;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum WitnessTreeEvent {
    UpdateBestTip {
        best_tip: Block,
        canonical_blocks: Vec<Block>,
    },
}

impl std::fmt::Debug for WitnessTreeEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UpdateBestTip {
                best_tip,
                canonical_blocks,
            } => {
                let canonical_blocks: Vec<String> =
                    canonical_blocks.iter().map(|b| b.summary()).collect();
                write!(
                    f,
                    "best_tip: {}\ncanonical_blocks: {:?}",
                    best_tip.summary(),
                    canonical_blocks
                )
            }
        }
    }
}
