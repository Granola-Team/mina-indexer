//! State events
//!
//! State events are not recorded in the event log

use crate::block::Block;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum StateEvent {
    UpdateCanonicalChain(Vec<Block>),
}

impl StateEvent {
    pub fn empty() -> Self {
        Self::UpdateCanonicalChain(vec![])
    }
}

impl std::fmt::Debug for StateEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UpdateCanonicalChain(blocks) => write!(f, "{:?}", blocks),
        }
    }
}
