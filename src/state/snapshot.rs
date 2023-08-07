use std::collections::HashMap;

use serde_derive::{Deserialize, Serialize};

use crate::block::BlockHash;

use super::{branch::Branch, ledger::diff::LedgerDiff};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StateSnapshot {
    pub root_branch: Branch,
    pub diffs_map: HashMap<BlockHash, LedgerDiff>,
}

pub trait StateStore {
    fn store_state_snapshot(&self, snapshot: &StateSnapshot) -> anyhow::Result<()>;
    fn read_snapshot(&self) -> anyhow::Result<Option<StateSnapshot>>;
}
