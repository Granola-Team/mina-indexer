use serde::{Deserialize, Serialize};

pub mod canonical_chain_discovery;
pub mod store;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum Canonicity {
    Canonical,
    Orphaned,
    Pending,
}
