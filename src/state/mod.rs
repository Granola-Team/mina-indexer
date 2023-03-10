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

pub enum ExtensionDirection {
    Forward,
    Reverse,
}

impl State {
    pub fn add_block(&mut self, precomputed_block: &PrecomputedBlock) -> Option<ExtensionType> {
        // forward extension on root branch
        if let Some(new_node_id) = self.root_branch.simple_extension(precomputed_block) {
            let mut branches_to_remove = Vec::new();
            for (index, dangling_branch) in self.dangling_branches.iter_mut().enumerate() {
                if precomputed_block.state_hash == dangling_branch.root.parent_hash.block_hash {
                    self.root_branch.merge_on(&new_node_id, dangling_branch);
                    branches_to_remove.push(index);
                }
            }

            if !branches_to_remove.is_empty() {
                for index_to_remove in branches_to_remove {
                    self.dangling_branches.remove(index_to_remove);
                }
                return Some(ExtensionType::RootComplex);
            } else {
                return Some(ExtensionType::RootSimple);
            }
        }

        let mut extension = None;
        for (index, dangling_branch) in self.dangling_branches.iter_mut().enumerate() {
            // simple reverse
            if precomputed_block.state_hash == dangling_branch.root.parent_hash.block_hash {
                dangling_branch.new_root(precomputed_block);
                extension = Some((
                    index,
                    dangling_branch
                        .branches
                        .root_node_id()
                        .expect("has root")
                        .clone(),
                    ExtensionDirection::Reverse,
                ));
                break;
            }

            if let Some(new_node_id) = dangling_branch.simple_extension(precomputed_block) {
                extension = Some((index, new_node_id, ExtensionDirection::Forward));
                break;
            }
        }

        if let Some((extended_branch_index, new_node_id, direction)) = extension {
            let mut branches_to_update = Vec::new();
            for (index, dangling_branch) in self.dangling_branches.iter().enumerate() {
                if precomputed_block.state_hash == dangling_branch.root.parent_hash.block_hash {
                    branches_to_update.push(index);
                }
            }

            if !branches_to_update.is_empty() {
                let mut extended_branch = self.dangling_branches.remove(extended_branch_index);
                for dangling_branch_index in branches_to_update {
                    let branch_to_update = self
                        .dangling_branches
                        .get_mut(dangling_branch_index)
                        .unwrap();
                    extended_branch.merge_on(&new_node_id, branch_to_update);
                    self.dangling_branches.remove(dangling_branch_index);
                }
                self.dangling_branches.push(extended_branch);
                return Some(ExtensionType::DanglingComplex);
            } else {
                return Some(match direction {
                    ExtensionDirection::Forward => ExtensionType::DanglingSimpleForward,
                    ExtensionDirection::Reverse => ExtensionType::DanglingSimpleReverse,
                });
            }
        }

        None
    }
}
