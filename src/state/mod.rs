use log::info;
use tracing::error;

use crate::block::{precomputed::PrecomputedBlock, store::BlockStoreConn, BlockHash};

use self::{
    branch::Branch,
    ledger::{command::Command, diff::LedgerDiff, genesis::GenesisLedger, Ledger},
};

pub mod branch;
pub mod ledger;

/// Rooted forest of precomputed block summaries
/// `root_branch` - represents the tree of blocks connecting back to a known ledger state, e.g. genesis
/// `dangling_branches` - trees of blocks stemming from an unknown ledger state
pub struct IndexerState {
    /// Append-only tree of blocks built from genesis, each containing a ledger
    pub root_branch: Branch<Ledger>,
    /// Dynamic, dangling branches eventually merged into the `root_branch`
    pub dangling_branches: Vec<Branch<LedgerDiff>>,
    /// Block database
    pub block_store: Option<BlockStoreConn>,
    pub transition_frontier_length: Option<u32>,
    pub prune_interval: Option<u32>,
    pub blocks_processed: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ExtensionType {
    DanglingNew,
    DanglingSimpleForward,
    DanglingSimpleReverse,
    DanglingComplex,
    RootSimple,
    RootComplex,
    BlockNotAdded,
}

pub enum ExtensionDirection {
    Forward,
    Reverse,
}

impl IndexerState {
    pub fn new(
        root_hash: BlockHash,
        genesis_ledger: GenesisLedger,
        rocksdb_path: Option<&std::path::Path>,
        transition_frontier_length: Option<u32>,
        prune_interval: Option<u32>,
    ) -> anyhow::Result<Self> {
        let block_store = rocksdb_path.map(|path| BlockStoreConn::new(path).unwrap());
        Ok(Self {
            root_branch: Branch::new_genesis(root_hash, Some(genesis_ledger)),
            dangling_branches: Vec::new(),
            block_store,
            transition_frontier_length,
            prune_interval,
            blocks_processed: 0,
        })
    }

    /// Adds the block to the witness tree and the precomputed block to the db
    ///
    /// Errors if the block is already present in the witness tree
    pub fn add_block(
        &mut self,
        precomputed_block: &PrecomputedBlock,
    ) -> anyhow::Result<ExtensionType> {
        // prune the root branch
        if let Some(k) = self.transition_frontier_length {
            if let Some(interval) = self.prune_interval {
                if self.blocks_processed % interval == 0 {
                    info!("pruning transition frontier at k={}", k);
                    self.root_branch.prune_transition_frontier(k);
                }
            } else {
                info!("pruning transition frontier at k={}", k);
                self.root_branch.prune_transition_frontier(k);
            }
        }

        // check that the block doesn't already exist in the db
        let state_hash = &precomputed_block.state_hash;
        if let Some(block_store) = self.block_store.as_ref() {
            match block_store.get_block(state_hash) {
                Err(err) => return Err(err),
                // add precomputed block to the db
                Ok(None) => block_store.add_block(precomputed_block)?,
                // log duplicate block to error
                Ok(_) => {
                    error!( "Block with state hash '{state_hash:?}' is already present in the block store");
                    return Ok(ExtensionType::BlockNotAdded);
                }
            }
        }

        self.blocks_processed += 1;

        // forward extension on root branch
        if let Some(_max_length) = self
            .root_branch
            .leaves
            .iter()
            .flat_map(|(_, x)| x.block.blockchain_length)
            .fold(None, |acc, x| acc.max(Some(x)))
        {
            // check leaf heights first
            // if max_length + 1 >= precomputed_block.blockchain_length.unwrap_or(0) {
            if let Some(new_node_id) = self.root_branch.simple_extension(precomputed_block) {
                let mut branches_to_remove = Vec::new();
                // check if new block connects to a dangling branch
                for (index, dangling_branch) in self.dangling_branches.iter_mut().enumerate() {
                    // incoming block is the parent of the dangling branch root
                    if precomputed_block.state_hash == dangling_branch.root.parent_hash.0 {
                        self.root_branch.merge_on(&new_node_id, dangling_branch);
                        branches_to_remove.push(index);
                    }

                    // if the block is already in the dangling branch, do nothing
                    if precomputed_block.state_hash == dangling_branch.root.state_hash.0 {
                        return Ok(ExtensionType::BlockNotAdded);
                    }
                }

                if !branches_to_remove.is_empty() {
                    // if the root branch is newly connected to dangling branches
                    for (num_removed, index_to_remove) in branches_to_remove.iter().enumerate() {
                        self.dangling_branches.remove(index_to_remove - num_removed);
                    }
                    return Ok(ExtensionType::RootComplex);
                } else {
                    // if there aren't any branches that are connected
                    return Ok(ExtensionType::RootSimple);
                }
            }
            // }
        }

        let mut extension = None;
        for (index, dangling_branch) in self.dangling_branches.iter_mut().enumerate() {
            match (
                dangling_branch
                    .leaves
                    .iter()
                    .flat_map(|(_, x)| x.block.blockchain_length)
                    .fold(None, |acc, x| acc.max(Some(x))),
                dangling_branch.root.blockchain_length,
            ) {
                (Some(max_length), min_length) => {
                    // check incoming block is within the height bounds
                    if let Some(length) = precomputed_block.blockchain_length {
                        if max_length + 1 >= length && length + 1 >= min_length.unwrap_or(0) {
                            // simple reverse
                            if precomputed_block.state_hash == dangling_branch.root.parent_hash.0 {
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
                            if let Some(new_node_id) =
                                dangling_branch.simple_extension(precomputed_block)
                            {
                                extension = Some((index, new_node_id, ExtensionDirection::Forward));
                                break;
                            }

                            // if the block is already in a dangling branch, do nothing
                            if precomputed_block.state_hash == dangling_branch.root.state_hash.0 {
                                return Ok(ExtensionType::BlockNotAdded);
                            }
                        }
                    } else {
                        // we don't know the blockchain_length for the incoming block, so we can't discriminate

                        // simple reverse
                        if precomputed_block.state_hash == dangling_branch.root.parent_hash.0 {
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

                        // if the block is already in a dangling branch, do nothing
                        if precomputed_block.state_hash == dangling_branch.root.state_hash.0 {
                            return Ok(ExtensionType::BlockNotAdded);
                        }

                        // simple forward
                        if let Some(new_node_id) =
                            dangling_branch.simple_extension(precomputed_block)
                        {
                            extension = Some((index, new_node_id, ExtensionDirection::Forward));
                            break;
                        }
                    }
                }
                (None, None) => {
                    // simple reverse
                    if precomputed_block.state_hash == dangling_branch.root.parent_hash.0 {
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

                    // if the block is already in a dangling branch, do nothing
                    if precomputed_block.state_hash == dangling_branch.root.state_hash.0 {
                        return Ok(ExtensionType::BlockNotAdded);
                    }

                    // simple forward
                    if let Some(new_node_id) = dangling_branch.simple_extension(precomputed_block) {
                        extension = Some((index, new_node_id, ExtensionDirection::Forward));
                        break;
                    }
                }
                _ => unreachable!(),
            }
        }

        // if a dangling branch has been extended (forward or reverse) check for new connections to other dangling branches
        if let Some((extended_branch_index, new_node_id, direction)) = extension {
            let mut branches_to_update = Vec::new();
            for (index, dangling_branch) in self.dangling_branches.iter().enumerate() {
                if precomputed_block.state_hash == dangling_branch.root.parent_hash.0 {
                    branches_to_update.push(index);
                }
            }

            if !branches_to_update.is_empty() {
                // remove one
                let mut extended_branch = self.dangling_branches.remove(extended_branch_index);
                for (n, dangling_branch_index) in branches_to_update.iter().enumerate() {
                    let index = if extended_branch_index < *dangling_branch_index {
                        dangling_branch_index - n - 1
                    } else {
                        *dangling_branch_index
                    };
                    let branch_to_update = self.dangling_branches.get_mut(index).unwrap();
                    extended_branch.merge_on(&new_node_id, branch_to_update);
                    // remove one for each index we see, i.e. n
                    self.dangling_branches.remove(index);
                }
                self.dangling_branches.push(extended_branch);
                return Ok(ExtensionType::DanglingComplex);
            } else {
                return match direction {
                    ExtensionDirection::Forward => Ok(ExtensionType::DanglingSimpleForward),
                    ExtensionDirection::Reverse => Ok(ExtensionType::DanglingSimpleReverse),
                };
            }
        }

        // block is added as a new dangling branch
        self.dangling_branches.push(
            Branch::new(
                precomputed_block,
                LedgerDiff::from_precomputed_block(precomputed_block),
            )
            .expect("cannot fail"),
        );
        Ok(ExtensionType::DanglingNew)
    }

    pub fn chain_commands(&self) -> Vec<Command> {
        if let Some(block_store) = self.block_store.as_ref() {
            return self
                .root_branch
                .longest_chain()
                .iter()
                .flat_map(|state_hash| block_store.get_block(&state_hash.0))
                .flatten()
                .flat_map(|precomputed_block| Command::from_precomputed_block(&precomputed_block))
                .collect();
        }
        vec![]
    }

    pub fn best_ledger(&self) -> Option<&Ledger> {
        if let Some(head_state_hash) = self.root_branch.longest_chain().first() {
            // find the corresponding leaf ledger
            for leaf in self.root_branch.leaves.values() {
                if &leaf.block.state_hash == head_state_hash {
                    return Some(leaf.get_ledger());
                }
            }
        }
        None
    }
}

impl std::fmt::Debug for IndexerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== State ===")?;

        writeln!(f, "Root branch:")?;
        let mut tree = String::new();
        self.root_branch.branches.write_formatted(&mut tree)?;
        writeln!(f, "{tree}")?;

        writeln!(f, "Dangling branches:")?;
        for (n, branch) in self.dangling_branches.iter().enumerate() {
            writeln!(f, "Dangling branch {n}:")?;
            let mut tree = String::new();
            branch.branches.write_formatted(&mut tree)?;
            write!(f, "{tree}")?;
        }
        Ok(())
    }
}
