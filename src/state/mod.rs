use crate::block::{precomputed::PrecomputedBlock, store::BlockStore, Block, BlockHash};

use self::{
    branch::{Branch, Leaf, RootedLeaf},
    ledger::{command::Command, diff::LedgerDiff, Ledger},
};

pub mod branch;
pub mod ledger;

#[derive(Debug)]
pub struct State {
    pub best_chain: Vec<RootedLeaf>,
    pub root_branch: Option<Branch<Ledger>>,
    pub dangling_branches: Vec<Branch<LedgerDiff>>,
    pub store: Option<BlockStore>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ExtensionType {
    DanglingNew,
    DanglingSimpleForward,
    DanglingSimpleReverse,
    DanglingComplex,
    RootNew,
    RootSimple,
    RootComplex,
}

pub enum ExtensionDirection {
    Forward,
    Reverse,
}

impl State {
    pub fn new(
        root: &PrecomputedBlock,
        blocks_path: Option<&std::path::Path>,
    ) -> anyhow::Result<Self> {
        let store = blocks_path.map(|path| BlockStore::new(path).unwrap());
        // genesis block => make new root_branch
        if root.state_hash == BlockHash::previous_state_hash(root).block_hash {
            // TODO get genesis ledger
            let genesis_ledger = Ledger::default();
            let block = Block::from_precomputed(root, 1);
            Ok(Self {
                best_chain: Vec::from([Leaf::new(block, genesis_ledger)]),
                root_branch: Some(Branch::<Ledger>::new_rooted(root)),
                dangling_branches: Vec::new(),
                store,
            })
        } else {
            Ok(Self {
                best_chain: Vec::new(),
                root_branch: None,
                dangling_branches: Vec::from([Branch::<LedgerDiff>::new_rooted(root)]),
                store,
            })
        }
    }

    pub fn add_block(&mut self, precomputed_block: &PrecomputedBlock) -> ExtensionType {
        // forward extension on root branch
        if let Some(mut root_branch) = self.root_branch.clone() {
            if let Some(new_node_id) = root_branch.simple_extension(precomputed_block) {
                let mut branches_to_remove = Vec::new();
                // check if new block connects to a dangling branch
                for (index, dangling_branch) in self.dangling_branches.iter_mut().enumerate() {
                    if precomputed_block.state_hash == dangling_branch.root.parent_hash.block_hash {
                        root_branch.merge_on(&new_node_id, dangling_branch);
                        branches_to_remove.push(index);
                    }
                }

                // should be the only place we need this call
                self.best_chain = root_branch.longest_chain();
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

    pub fn chain_commands(&self) -> Vec<Command> {
        self.best_chain
            .iter()
            .map(|leaf| leaf.block.state_hash.clone())
            .flat_map(|state_hash| {
                self.store
                    .as_ref()
                    .unwrap()
                    .get_block(&state_hash.block_hash)
            })
            .flatten()
            .flat_map(|precomputed_block| Command::from_precomputed_block(&precomputed_block))
            .collect()
    }
}
