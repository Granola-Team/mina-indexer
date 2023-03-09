use crate::block::{precomputed::PrecomputedBlock, store::BlockStore};

use self::branch::{Branch, Path};

pub mod ledger;
// pub mod best_tip;
// pub mod block;
pub mod branch;
// pub mod store;
// pub mod voting;

// use self::head::Head;
// use self::block::{Block, BlockHash};
// use self::branch::{Branches, Leaves};
// use self::store::Store;

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Head {}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Store {}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Status {}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct DanglingBranches {}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct StateUpdate {}

pub type RefLog = Vec<StateUpdate>;

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

pub enum ExtensionType {
    DanglingSimpleForward,
    DanglingSimpleReverse,
    DanglingComplex,
    RootSimple,
    RootComplex,
}

impl State {
    pub fn add_block(&mut self, precomputed_block: &PrecomputedBlock) -> Option<ExtensionType> {
        // forward extension on root branch
        if let Some(_new_node_id) = self.root_branch.simple_extension(precomputed_block) {
            for _dangling_branch in self.dangling_branches.iter_mut() {}
            return Some(ExtensionType::RootSimple);
        }

        None
    }
}
