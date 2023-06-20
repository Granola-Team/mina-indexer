use crate::{
    block::{
        parser::BlockParser, precomputed::PrecomputedBlock, Block, BlockHash, store::BlockStore,
    },
    state::{
        branch::Branch,
        ledger::{command::Command, genesis::GenesisLedger, Ledger},
    },
    store::IndexerStore,
    BLOCK_REPORTING_FREQ, PRUNE_INTERVAL_DEFAULT, MAINNET_TRANSITION_FRONTIER_K,
};
use id_tree::NodeId;
use std::{path::Path, time::Instant, borrow::Borrow};
use time::OffsetDateTime;
use tracing::{debug, info, warn};

use self::ledger::{diff::LedgerDiff, store::LedgerStore};

pub mod branch;
pub mod ledger;
pub mod summary;

/// Rooted forest of precomputed block summaries aka the witness tree
/// `root_branch` - represents the tree of blocks connecting back to a known ledger state, e.g. genesis
/// `dangling_branches` - trees of blocks stemming from an unknown ledger state
pub struct IndexerState {
    /// Indexer mode
    pub mode: IndexerMode,
    /// Indexer phase
    pub phase: IndexerPhase,
    /// Block representing the best tip of the root branch
    pub best_tip: Block,
    /// Append-only tree of blocks built from genesis, each containing a ledger
    pub root_branch: Branch,
    /// Dynamic, dangling branches eventually merged into the `root_branch`
    /// needed for the possibility of missing blocks
    pub dangling_branches: Vec<Branch>,
    /// Block database
    pub indexer_store: Option<IndexerStore>,
    /// Threshold amount of confirmations to trigger a pruning event
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

#[derive(Debug, Clone)]
pub enum IndexerPhase {
    InitializingFromBlockDir,
    InitializingFromDB,
    Watching,
    Testing,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum IndexerMode {
    Light,
    Full,
    Test,
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
        mode: IndexerMode,
        root_hash: BlockHash,
        genesis_ledger: GenesisLedger,
        rocksdb_path: Option<&Path>,
        transition_frontier_length: Option<u32>,
        prune_interval: Option<u32>,
    ) -> anyhow::Result<Self> {
        let root_branch = Branch::new_genesis(root_hash.clone());
        let indexer_store = rocksdb_path.map(|path| {
            let store = IndexerStore::new(path).unwrap();
            store
                .add_ledger(&root_hash, genesis_ledger.into())
                .expect("ledger add succeeds");
            store
        });
        Ok(Self {
            mode,
            phase: IndexerPhase::InitializingFromBlockDir,
            best_tip: root_branch.root.clone(),
            root_branch,
            dangling_branches: Vec::new(),
            indexer_store,
            transition_frontier_length,
            prune_interval,
            blocks_processed: 0,
            time: Instant::now(),
            date_time: OffsetDateTime::now_utc(),
        })
    }

    pub fn new_non_genesis(
        mode: IndexerMode,
        root_hash: BlockHash,
        ledger: Ledger,
        blockchain_length: Option<u32>,
        rocksdb_path: Option<&Path>,
        transition_frontier_length: Option<u32>,
        prune_interval: Option<u32>,
    ) -> anyhow::Result<Self> {
        let root_branch = Branch::new_non_genesis(root_hash.clone(), blockchain_length);
        let indexer_store = rocksdb_path.map(|path| {
            let store = IndexerStore::new(path).unwrap();
            store
                .add_ledger(&root_hash, ledger)
                .expect("ledger add succeeds");
            store
        });
        Ok(Self {
            mode,
            phase: IndexerPhase::InitializingFromDB,
            best_tip: root_branch.root.clone(),
            root_branch,
            dangling_branches: Vec::new(),
            indexer_store,
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
        rocksdb_path: Option<&Path>,
        transition_frontier_length: Option<u32>,
    ) -> anyhow::Result<Self> {
        let root_branch = Branch::new_testing(root_block);
        let indexer_store = rocksdb_path.map(|path| {
            let store = IndexerStore::new(path).unwrap();
            if let Some(ledger) = root_ledger {
                store
                    .add_ledger(&BlockHash(root_block.state_hash.clone()), ledger)
                    .expect("ledger add succeeds");
            }
            store
        });
        Ok(Self {
            mode: IndexerMode::Test,
            phase: IndexerPhase::Testing,
            best_tip: root_branch.root.clone(),
            root_branch,
            dangling_branches: Vec::new(),
            indexer_store,
            transition_frontier_length,
            prune_interval: None,
            blocks_processed: 0,
            time: Instant::now(),
            date_time: OffsetDateTime::now_utc(),
        })
    }

    pub fn restore_from_db(db: &IndexerStore) -> anyhow::Result<Self> {
        // TODO
        // find best tip block in db (according to Block::cmp)
        // go back at least 290 blocks (make this block the root of the root tree)
        // Q: How to compute ledger? Should we require it to do quick sync?
        // iterate over the db blocks, starting at the root, adding them to the state with add_block(..., false)
        let mut state_opt: Option<IndexerState> = None;
        for res in db.iterator() {
            match res {
                Ok((_k, v)) => {
                    if let Ok(block) = bcs::from_bytes::<PrecomputedBlock>(v.borrow()) {
                        if let Some(mut state) = state_opt {
                            state.add_block(&block, false).unwrap();
                            state_opt = Some(state);
                        } else {
                            state_opt = Some(
                                Self::new_testing(
                                    &block,
                                    None,
                                    Some(db.db_path()),
                                    Some(MAINNET_TRANSITION_FRONTIER_K),
                                )
                                .unwrap(),
                            )
                        }
                    }
                }
                Err(_e) => (),
            }
        }
        todo!()
    }

    fn prune_root_branch(&mut self) {
        if let Some(k) = self.transition_frontier_length {
            let interval = self.prune_interval.unwrap_or(PRUNE_INTERVAL_DEFAULT);
            if self.root_branch.height() as u32 > interval * k {
                debug!(
                    "Pruning transition frontier at k = {}, best tip length: {}",
                    k,
                    self.best_tip.blockchain_length.unwrap_or(0)
                );
                self.root_branch
                    .prune_transition_frontier(k, &self.best_tip);
            }
        }
    }

    /// Adds blocks to the state according to block_parser then changes phase to Watching
    ///
    /// Returns the number of blocks parsed
    pub async fn add_blocks(&mut self, block_parser: &mut BlockParser) -> anyhow::Result<u32> {
        let mut block_count = 0;
        let time = Instant::now();

        while let Some(block) = block_parser.next().await? {
            if block_count > 0 && block_count % BLOCK_REPORTING_FREQ == 0 {
                info!(
                    "{}",
                    format!(
                        "Parsed and added {block_count} blocks to the witness tree in {:?}",
                        time.elapsed()
                    )
                );
            }
            self.add_block(&block, true)?;
            block_count += 1;
        }

        debug!("Phase change: {} -> {}", self.phase, IndexerPhase::Watching);
        self.phase = IndexerPhase::Watching;
        Ok(block_count)
    }

    /// Adds the block to the witness tree and the precomputed block to the db
    ///
    /// Errors if the block is already present in the witness tree
    pub fn add_block(
        &mut self,
        precomputed_block: &PrecomputedBlock,
        check_if_block_in_db: bool,
    ) -> anyhow::Result<ExtensionType> {
        self.prune_root_branch();

        // check that the block doesn't already exist in the db
        if check_if_block_in_db && self.block_exists(&precomputed_block.state_hash)? {
            warn!(
                "Block with state hash '{:?}' is already present in the block store",
                &precomputed_block.state_hash
            );
            return Ok(ExtensionType::BlockNotAdded);
        }

        if let Some(indexer_store) = self.indexer_store.as_ref() {
            indexer_store.add_block(precomputed_block)?;
        }
        self.blocks_processed += 1;

        // forward extension on root branch
        // check leaf heights first
        if (precomputed_block.blockchain_length.is_some()
            && self.best_tip.blockchain_length.unwrap_or(0) + 1
                >= precomputed_block.blockchain_length.unwrap())
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
            return self.update_dangling(
                precomputed_block,
                extended_branch_index,
                new_node_id,
                direction,
            );
        }

        // block is added as a new dangling branch
        self.new_dangling(precomputed_block)
    }

    pub fn block_exists(&self, state_hash: &str) -> anyhow::Result<bool> {
        if let Some(indexer_store) = self.indexer_store.as_ref() {
            match indexer_store.get_block(&BlockHash(state_hash.to_string()))? {
                None => Ok(false),
                Some(block) => {
                    warn!(
                        "Attempted to add duplicate block! length: {}, state_hash: {:?}",
                        block.blockchain_length.unwrap_or(0),
                        block.state_hash
                    );
                    Ok(true)
                }
                Some(_block) => Ok(true),
            }
        } else {
            Ok(false)
        }
    }

    pub fn root_extension(
        &mut self,
        precomputed_block: &PrecomputedBlock,
    ) -> anyhow::Result<Option<ExtensionType>> {
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
        } else {
            Ok(None)
        }
    }

    pub fn dangling_extension(
        &mut self,
        precomputed_block: &PrecomputedBlock,
    ) -> anyhow::Result<Option<(usize, NodeId, ExtensionDirection)>> {
        let mut extension = None;
        for (index, dangling_branch) in self.dangling_branches.iter_mut().enumerate() {
            let min_length = dangling_branch.root.blockchain_length.unwrap_or(0);
            let max_length = dangling_branch
                .best_tip()
                .unwrap()
                .blockchain_length
                .unwrap_or(0);
            // check incoming block is within the length bounds
            if let Some(length) = precomputed_block.blockchain_length {
                if max_length + 1 >= length && length + 1 >= min_length {
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
                if let Some(new_node_id) = dangling_branch.simple_extension(precomputed_block) {
                    extension = Some((index, new_node_id, ExtensionDirection::Forward));
                    break;
                }
            }
        }

        Ok(extension)
    }

    pub fn update_dangling(
        &mut self,
        precomputed_block: &PrecomputedBlock,
        extended_branch_index: usize,
        new_node_id: NodeId,
        direction: ExtensionDirection,
    ) -> anyhow::Result<ExtensionType> {
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
            Ok(ExtensionType::DanglingComplex)
        } else {
            match direction {
                ExtensionDirection::Forward => Ok(ExtensionType::DanglingSimpleForward),
                ExtensionDirection::Reverse => Ok(ExtensionType::DanglingSimpleReverse),
            }
        }
    }

    pub fn new_dangling(
        &mut self,
        precomputed_block: &PrecomputedBlock,
    ) -> anyhow::Result<ExtensionType> {
        self.dangling_branches
            .push(Branch::new(precomputed_block).expect("cannot fail"));
        Ok(ExtensionType::DanglingNew)
    }

    pub fn chain_commands(&self) -> Vec<Command> {
        if let Some(indexer_store) = self.indexer_store.as_ref() {
            return self
                .root_branch
                .longest_chain()
                .iter()
                .flat_map(|state_hash| indexer_store.get_block(state_hash))
                .flatten()
                .flat_map(|precomputed_block| Command::from_precomputed_block(&precomputed_block))
                .collect();
        }
        vec![]
    }

    pub fn best_ledger(&self) -> anyhow::Result<Ledger> {
        let mut ledger = None;
        if let (Some(block), Some(store)) =
            (self.root_branch.best_tip(), self.indexer_store.as_ref())
        {
            ledger = store.get_ledger(&block.state_hash)?;
            if ledger.is_none() {
                let mut ledger_diffs = Vec::new();
                let mut state_hash = block.state_hash;
                loop {
                    if let Some(block_ledger) = store.get_ledger(&state_hash)? {
                        ledger = Some(block_ledger);
                        break;
                    }
                    let precomputed_block = store
                        .get_block(&state_hash)?
                        .expect("block comes from root branch, is in block store");
                    let ledger_diff = LedgerDiff::from_precomputed_block(&precomputed_block);
                    ledger_diffs.push(ledger_diff);
                    state_hash = BlockHash::from_hashv1(
                        precomputed_block.protocol_state.previous_state_hash,
                    );
                }
                ledger_diffs.into_iter().for_each(|diff| {
                    ledger.iter_mut().for_each(|ledger| {
                        ledger
                            .apply_diff(diff.clone())
                            .expect("ledger diff application succeeds")
                    })
                });

                if let Some(ledger) = ledger.clone() {
                    store
                        .add_ledger(&state_hash, ledger)
                        .expect("ledger add succeeds")
                }
            }
        }

        Ok(ledger.expect("genesis ledger guaranteed"))
    }

    fn update_best_tip(&mut self) {
        if self.root_branch.best_tip().is_none() {
            println!("~~~ Root tree ~~~");
            println!("{:?}", self.root_branch);
        }
        self.best_tip = self.root_branch.best_tip().unwrap();
    }

    fn same_block_added_twice(
        branch: &Branch,
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

impl std::fmt::Display for IndexerMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexerMode::Full => write!(f, "full"),
            IndexerMode::Light => write!(f, "light"),
            IndexerMode::Test => write!(f, "test"),
        }
    }
}

impl std::fmt::Display for IndexerPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexerPhase::InitializingFromBlockDir | IndexerPhase::InitializingFromDB => {
                write!(f, "initializing")
            }
            IndexerPhase::Watching => write!(f, "watching"),
            IndexerPhase::Testing => write!(f, "testing"),
        }
    }
}
