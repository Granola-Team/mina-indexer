use crate::block::{precomputed::PrecomputedBlock, store::BlockStore, Block, BlockHash};

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
    pub root: BlockHash,
    pub best_chain: Path,
    pub branches: Vec<Branch>,
    pub store: BlockStore,
}

impl State {
    pub fn new(root: BlockHash, blocks_path: &std::path::Path) -> Result<Self, anyhow::Error> {
        let best_chain = Vec::new();
        let branches = Vec::new();
        let store = BlockStore::new(blocks_path)?;
        Ok(Self {
            root,
            best_chain,
            branches,
            store,
        })
    }

    pub fn add_block(&mut self, block: PrecomputedBlock) -> Result<(), anyhow::Error> {
        self.store.add_block(&block)?;
        for branch in self.branches.iter_mut() {
            branch.try_add_block(&Block::from_precomputed(&block))?;
            let longest_path = branch.longest_path();
            if let Some(root_block) = longest_path.first() {
                if root_block.state_hash == self.root && longest_path.len() >= self.best_chain.len()
                {
                    self.best_chain = longest_path;
                }
            }
        }

        Ok(())
    }
}
