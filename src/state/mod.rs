use crate::block::{precomputed::PrecomputedBlock, store::BlockStore};

use self::branch::{Branch, Path};

pub mod branch;
pub mod ledger;

#[derive(Debug)]
pub struct State {
    pub best_chain: Path,
    pub root_branch: Branch,
    pub dangling_branches: Vec<Branch>,
    pub store: BlockStore,
}

impl State {
    pub fn new(
        root: &PrecomputedBlock,
        blocks_path: &std::path::Path,
    ) -> Result<Self, anyhow::Error> {
        let best_chain = Vec::new();
        let root_branch = Branch::new(root)?;
        let dangling_branches = Vec::new();
        let store = BlockStore::new(blocks_path)?;
        Ok(Self {
            best_chain,
            root_branch,
            dangling_branches,
            store,
        })
    }
}
