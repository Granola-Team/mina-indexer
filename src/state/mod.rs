use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStoreConn, Block, BlockHash},
    state::{
        branch::Branch,
        ledger::{command::Command, diff::LedgerDiff, genesis::GenesisLedger, Ledger},
    },
};
use std::time::Instant;
use id_tree::NodeId;
use time::OffsetDateTime;
use tracing::{info, warn};

pub mod branch;
pub mod ledger;
pub mod summary;

/// Rooted forest of precomputed block summaries
/// `root_branch` - represents the tree of blocks connecting back to a known ledger state, e.g. genesis
/// `dangling_branches` - trees of blocks stemming from an unknown ledger state
pub struct IndexerState {
    /// State hahs of the best tip of the root branch
    pub best_tip: Block,
    /// Append-only tree of blocks built from genesis, each containing a ledger
    pub root_branch: Branch<Ledger>,
    /// Dynamic, dangling branches eventually merged into the `root_branch`
    pub dangling_branches: Vec<Branch<LedgerDiff>>,
    /// Block database
    pub block_store: Option<BlockStoreConn>,
    pub transition_frontier_length: Option<u32>,
    /// Interval to the prune the root branch
    pub prune_interval: Option<u32>,
    /// Number of blocks added to the state
    pub blocks_processed: u32,
    /// Time the indexer started running
    pub time: Instant,
    /// Datetime the indexer started running
    pub date_time: OffsetDateTime,
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
        let root_branch = Branch::new_genesis(root_hash, Some(genesis_ledger));
        let block_store = rocksdb_path.map(|path| BlockStoreConn::new(path).unwrap());
        Ok(Self {
            best_tip: root_branch.root.clone(),
            root_branch,
            dangling_branches: Vec::new(),
            block_store,
            transition_frontier_length,
            prune_interval,
            blocks_processed: 0,
            time: Instant::now(),
            date_time: OffsetDateTime::now_utc(),
        })
    }

    pub fn new_testing(
        root_block: &PrecomputedBlock,
        root_ledger: Option<Ledger>,
        rocksdb_path: Option<&std::path::Path>,
        transition_frontier_length: Option<u32>,
    ) -> anyhow::Result<Self> {
        let root_branch = Branch::new_testing(root_block, root_ledger);
        let block_store = rocksdb_path.map(|path| BlockStoreConn::new(path).unwrap());
        Ok(Self {
            best_tip: root_branch.root.clone(),
            root_branch,
            dangling_branches: Vec::new(),
            block_store,
            transition_frontier_length,
            prune_interval: None,
            blocks_processed: 0,
            time: Instant::now(),
            date_time: OffsetDateTime::now_utc(),
        })
    }

    fn prune_root_branch(&mut self) {
        if let Some(k) = self.transition_frontier_length {
            let interval = self.prune_interval.unwrap_or(5);
            if self.root_branch.height() as u32 > interval * k {
                info!("Pruning transition frontier at k={}", k);
                self.root_branch
                    .prune_transition_frontier(k, &self.best_tip);
            }
        }
    }

    /// Adds the block to the witness tree and the precomputed block to the db
    ///
    /// Errors if the block is already present in the witness tree
    pub fn add_block(
        &mut self,
        precomputed_block: &PrecomputedBlock,
    ) -> anyhow::Result<ExtensionType> {
        // prune the root branch
        self.prune_root_branch();

        // check that the block doesn't already exist in the db
        if self.block_exists(&precomputed_block.state_hash)? {
            warn!( "Block with state hash '{:?}' is already present in the block store", &precomputed_block.state_hash);
            return Ok(ExtensionType::BlockNotAdded);
        }

        if let Some(block_store) = self.block_store.as_ref() {
            block_store.add_block(precomputed_block)?;
        }
        self.blocks_processed += 1;

        // forward extension on root branch
        // check leaf heights first
        if precomputed_block.blockchain_length.is_some()
            && self.best_tip.blockchain_length.unwrap_or(0) + 1
                >= precomputed_block.blockchain_length.unwrap()
            || precomputed_block.blockchain_length.is_none()
        {
            if let Some(root_extension) = self.root_extension(precomputed_block)? {
                return Ok(root_extension);
            }
        }

        // if a dangling branch has been extended (forward or reverse) check for new connections to other dangling branches
        if let Some((extended_branch_index, new_node_id, direction)) = 
            self.dangling_extension(precomputed_block)? 
        {
            return Ok(self.update_dangling(
                precomputed_block, 
                extended_branch_index, 
                new_node_id, 
                direction)?
            );
        }

        // block is added as a new dangling branch
        self.new_dangling(precomputed_block)
    }

    pub fn block_exists(&self, state_hash: &str) -> anyhow::Result<bool> {
        if let Some(block_store) = self.block_store.as_ref() {
            match block_store.get_block(state_hash)? {
                None => Ok(false),
                // log duplicate block to error
                Some(_block) => Ok(true)
            }
        } else { Ok(false) } 
    }

    pub fn root_extension(&mut self, precomputed_block: &PrecomputedBlock) -> anyhow::Result<Option<ExtensionType>> {
        if let Some(new_node_id) = self.root_branch.simple_extension(precomputed_block) {
            self.update_best_tip();
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
                    return Err(anyhow::Error::msg(
                        "Same block added twice to the indexer state",
                    ));
                }
            }

            if !branches_to_remove.is_empty() {
                // if the root branch is newly connected to dangling branches
                for (num_removed, index_to_remove) in branches_to_remove.iter().enumerate() {
                    self.dangling_branches.remove(index_to_remove - num_removed);
                }

                self.update_best_tip();
                Ok(Some(ExtensionType::RootComplex))
            } else {
                // if there aren't any branches that are connected
                Ok(Some(ExtensionType::RootSimple))
            }
        } else { Ok(None) }
    }

    pub fn dangling_extension(&mut self, precomputed_block: &PrecomputedBlock) -> anyhow::Result<Option<(usize, NodeId, ExtensionDirection)>> {
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
                    // check incoming block is within the length bounds
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

                            Self::same_block_added_twice(dangling_branch, precomputed_block)?;
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

                        Self::same_block_added_twice(dangling_branch, precomputed_block)?;

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

                    // simple forward
                    if let Some(new_node_id) = dangling_branch.simple_extension(precomputed_block) {
                        extension = Some((index, new_node_id, ExtensionDirection::Forward));
                        break;
                    }
                }
                _ => unreachable!(),
            }
        }

        Ok(extension)
    }

    pub fn update_dangling(
        &mut self, 
        precomputed_block: &PrecomputedBlock,
        extended_branch_index: usize, 
        new_node_id: NodeId, 
        direction: ExtensionDirection) 
        -> anyhow::Result<ExtensionType>
    {
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

                    // remove one for each index we see
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

    pub fn new_dangling(&mut self, precomputed_block: &PrecomputedBlock) -> anyhow::Result<ExtensionType> {
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

    fn update_best_tip(&mut self) {
        if let Some(best_tip_leaf) = self
            .root_branch
            .leaves
            .values()
            .max_by(|leafx, leafy| leafx.block.cmp(&leafy.block))
        {
            self.best_tip = best_tip_leaf.block.clone();
        };
    }

    fn same_block_added_twice<T>(
        branch: &Branch<T>,
        precomputed_block: &PrecomputedBlock,
    ) -> anyhow::Result<()> {
        if precomputed_block.state_hash == branch.root.state_hash.0 {
            return Err(anyhow::Error::msg(
                "Same block added twice to the indexer state",
            ));
        }
        Ok(())
    }
}

impl std::fmt::Debug for IndexerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Root branch ===")?;
        writeln!(f, "{:?}", self.root_branch)?;

        writeln!(f, "=== Dangling branches ===")?;
        for (n, branch) in self.dangling_branches.iter().enumerate() {
            writeln!(f, "Dangling branch {n}:")?;
            writeln!(f, "{branch:?}")?;
        }
        Ok(())
    }
}
