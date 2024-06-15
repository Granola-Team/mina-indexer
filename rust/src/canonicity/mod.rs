pub mod canonical_chain_discovery;
pub mod store;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum Canonicity {
    Canonical,
    Orphaned,
    Pending,
}
