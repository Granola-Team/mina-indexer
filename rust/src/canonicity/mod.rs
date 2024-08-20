pub mod canonical_chain_discovery;
pub mod store;

use crate::{block::BlockHash, store::DbUpdate};
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct CanonicityDiff {
    pub state_hash: BlockHash,
    pub blockchain_length: u32,
    pub global_slot: u32,
}

pub type CanonicityUpdate = DbUpdate<CanonicityDiff>;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum Canonicity {
    Canonical,
    Orphaned,
    Pending,
}

impl std::fmt::Debug for CanonicityDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string_pretty(self).unwrap_or("{}".to_string())
        )
    }
}
