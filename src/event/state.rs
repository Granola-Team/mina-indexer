//! State events
//!
//! State events are not recorded in the event log

use crate::block::Block;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StateEvent {
    UpdateCanonicalChain(Vec<Block>),
}
