use crate::block::{precomputed::PrecomputedBlock, store::BlockStore};

use self::{
    branch::{Branch, RootedLeaf},
    ledger::{diff::LedgerDiff, Ledger},
};

pub mod branch;
pub mod ledger;

#[derive(Debug)]
pub struct State {
    pub best_chain: Vec<RootedLeaf>,
    pub root_branch: Branch<Ledger>,
    pub dangling_branches: Vec<Branch<LedgerDiff>>,
    pub store: BlockStore,
}

impl State {
    pub fn new(
        root: &PrecomputedBlock,
        blocks_path: &std::path::Path,
    ) -> Result<Self, anyhow::Error> {
        let best_chain = Vec::new();
        let root_branch = Branch::new_rooted(root);
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
    DanglingNew,
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
    pub fn add_block(&mut self, precomputed_block: &PrecomputedBlock) -> ExtensionType {
        // forward extension on root branch
        if let Some(new_node_id) = self.root_branch.simple_extension(precomputed_block) {
            let mut branches_to_remove = Vec::new();
            // check if new block connects to a dangling branch
            for (index, dangling_branch) in self.dangling_branches.iter_mut().enumerate() {
                if precomputed_block.state_hash == dangling_branch.root.parent_hash.block_hash {
                    self.root_branch.merge_on(&new_node_id, dangling_branch);
                    branches_to_remove.push(index);
                }
            }

            // should be the only place we need this call
            self.best_chain = self.root_branch.longest_chain();
            if !branches_to_remove.is_empty() {
                // if the root branch is newly connected to dangling branches
                for index_to_remove in branches_to_remove {
                    self.dangling_branches.remove(index_to_remove);
                }
                return ExtensionType::RootComplex;
            } else {
                // if there aren't any branches that are connected
                return ExtensionType::RootSimple;
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

            // simple forward
            if let Some(new_node_id) = dangling_branch.simple_extension(precomputed_block) {
                extension = Some((index, new_node_id, ExtensionDirection::Forward));
                break;
            }
        }

        // if a dangling branch has been extended (forward or reverse) check for new connections to other dangling branches
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
                return ExtensionType::DanglingComplex;
            } else {
                return match direction {
                    ExtensionDirection::Forward => ExtensionType::DanglingSimpleForward,
                    ExtensionDirection::Reverse => ExtensionType::DanglingSimpleReverse,
                };
            }
        }

        // block is added as a new dangling branch
        self.dangling_branches.push(
            Branch::new(
                precomputed_block,
                LedgerDiff::fom_precomputed_block(precomputed_block),
            )
            .expect("cannot fail"),
        );
        ExtensionType::DanglingNew
    }
}
