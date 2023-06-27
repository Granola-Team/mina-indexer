use self::summary::{
    DbStats, SummaryShort, SummaryVerbose, WitnessTreeSummaryShort, WitnessTreeSummaryVerbose,
};
use crate::{
    block::{
        parser::BlockParser, precomputed::PrecomputedBlock, store::BlockStore, Block, BlockHash,
        BlockWithoutHeight,
    },
    state::{
        branch::Branch,
        ledger::{
            command::Command, diff::LedgerDiff, genesis::GenesisLedger, store::LedgerStore, Ledger,
        },
    },
    store::IndexerStore,
    BLOCK_REPORTING_FREQ_NUM, BLOCK_REPORTING_FREQ_SEC, CANONICAL_UPDATE_THRESHOLD,
    MAINNET_CANONICAL_THRESHOLD, MAINNET_TRANSITION_FRONTIER_K, PRUNE_INTERVAL_DEFAULT,
};
use id_tree::NodeId;
use serde_derive::{Deserialize, Serialize};
use std::{
    borrow::Borrow,
    collections::HashMap,
    path::Path,
    str::FromStr,
    time::{Duration, Instant},
};
use time::{OffsetDateTime, PrimitiveDateTime};
use tracing::{debug, info, instrument, warn};

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
    pub best_tip: Tip,
    /// Highest known canonical block
    pub canonical_tip: Tip,
    /// Map of ledger diffs following the canonical tip
    pub diffs_map: HashMap<BlockHash, LedgerDiff>,
    /// Append-only tree of blocks built from genesis, each containing a ledger
    pub root_branch: Branch,
    /// Dynamic, dangling branches eventually merged into the `root_branch`
    /// needed for the possibility of missing blocks
    pub dangling_branches: Vec<Branch>,
    /// Block database
    pub indexer_store: Option<IndexerStore>,
    /// Threshold amount of confirmations to trigger a pruning event
    pub transition_frontier_length: u32,
    /// Interval to the prune the root branch
    pub prune_interval: u32,
    /// Threshold for updating the canonical tip and db ledger
    pub canonical_update_threshold: u32,
    /// Number of blocks added to the state
    pub blocks_processed: u32,
    /// Time the indexer started running
    pub time: Instant,
    /// Datetime the indexer started running
    pub date_time: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct Tip {
    pub state_hash: BlockHash,
    pub node_id: NodeId,
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum Canonicity {
    Canonical,
    Orphaned,
    Pending,
}

impl IndexerState {
    /// Creates a new indexer state from the genesis ledger
    pub fn new(
        mode: IndexerMode,
        root_hash: BlockHash,
        genesis_ledger: GenesisLedger,
        rocksdb_path: Option<&Path>,
        transition_frontier_length: u32,
        prune_interval: u32,
        canonical_update_threshold: u32,
    ) -> anyhow::Result<Self> {
        let root_branch = Branch::new_genesis(root_hash.clone());
        let indexer_store = rocksdb_path.map(|path| {
            let store = IndexerStore::new(path).unwrap();
            store
                .add_ledger(&root_hash, genesis_ledger.into())
                .expect("ledger add succeeds");
            store
        });
        let tip = Tip {
            state_hash: root_branch.root_block().state_hash.clone(),
            node_id: root_branch.root.clone(),
        };

        Ok(Self {
            mode,
            phase: IndexerPhase::InitializingFromBlockDir,
            canonical_tip: tip.clone(),
            diffs_map: HashMap::new(),
            best_tip: tip,
            root_branch,
            dangling_branches: Vec::new(),
            indexer_store,
            transition_frontier_length,
            prune_interval,
            canonical_update_threshold,
            blocks_processed: 0,
            time: Instant::now(),
            date_time: OffsetDateTime::now_utc(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    /// Creates a new indexer state from a "canonical" ledger
    pub fn new_non_genesis(
        mode: IndexerMode,
        root_hash: BlockHash,
        ledger: Ledger,
        blockchain_length: Option<u32>,
        rocksdb_path: Option<&Path>,
        transition_frontier_length: u32,
        prune_interval: u32,
        canonical_update_threshold: u32,
    ) -> anyhow::Result<Self> {
        let root_branch = Branch::new_non_genesis(root_hash.clone(), blockchain_length);
        let indexer_store = rocksdb_path.map(|path| {
            let store = IndexerStore::new(path).unwrap();
            store
                .add_ledger(&root_hash, ledger)
                .expect("ledger add succeeds");
            store
        });
        let tip = Tip {
            state_hash: root_branch.root_block().state_hash.clone(),
            node_id: root_branch.root.clone(),
        };

        Ok(Self {
            mode,
            phase: IndexerPhase::InitializingFromDB,
            canonical_tip: tip.clone(),
            diffs_map: HashMap::new(),
            best_tip: tip,
            root_branch,
            dangling_branches: Vec::new(),
            indexer_store,
            transition_frontier_length,
            prune_interval,
            canonical_update_threshold,
            blocks_processed: 0,
            time: Instant::now(),
            date_time: OffsetDateTime::now_utc(),
        })
    }

    /// Creates a new indexer state for testing
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
        let tip = Tip {
            state_hash: root_branch.root_block().state_hash.clone(),
            node_id: root_branch.root.clone(),
        };

        Ok(Self {
            mode: IndexerMode::Test,
            phase: IndexerPhase::Testing,
            canonical_tip: tip.clone(),
            diffs_map: HashMap::new(),
            best_tip: tip,
            root_branch,
            dangling_branches: Vec::new(),
            indexer_store,
            transition_frontier_length: transition_frontier_length
                .unwrap_or(MAINNET_TRANSITION_FRONTIER_K),
            prune_interval: PRUNE_INTERVAL_DEFAULT,
            canonical_update_threshold: CANONICAL_UPDATE_THRESHOLD,
            blocks_processed: 0,
            time: Instant::now(),
            date_time: OffsetDateTime::now_utc(),
        })
    }

    pub fn new_with_db(
        root_block: Block,
        canonical_update_threshold: u32,
        transition_frontier_length: Option<u32>,
        store: IndexerStore,
    ) -> anyhow::Result<Self> {
        let root_branch =
            Branch::new_non_genesis(root_block.state_hash.clone(), root_block.blockchain_length);
        let tip = Tip {
            state_hash: root_block.state_hash.clone(),
            node_id: root_branch.root.clone(),
        };

        Ok(Self {
            mode: IndexerMode::Light,
            phase: IndexerPhase::InitializingFromDB,
            canonical_tip: tip.clone(),
            diffs_map: HashMap::new(),
            best_tip: tip,
            root_branch,
            dangling_branches: Vec::new(),
            indexer_store: Some(store),
            transition_frontier_length: transition_frontier_length
                .unwrap_or(MAINNET_TRANSITION_FRONTIER_K),
            prune_interval: PRUNE_INTERVAL_DEFAULT,
            canonical_update_threshold,
            blocks_processed: 0,
            time: Instant::now(),
            date_time: OffsetDateTime::now_utc(),
        })
    }

    #[instrument]
    pub fn restore_from_db(db: IndexerStore, canonical_update_threshold: u32) -> anyhow::Result<Self> {
        // TODO
        // find best tip block in db (according to Block::cmp)
        // go back at least 290 blocks (make this block the root of the root tree)
        // Q: How to compute ledger? Should we require it to do quick sync?
        // iterate over the db blocks, starting at the root, adding them to the state with add_block(..., false)

        // find the best tip
        debug!("finding best tip in the provided database");
        let mut best_tip_length = u32::MIN;
        let mut best_tip_state_hash = BlockHash::from_bytes([0; 32]);
        for entry in db.iterator() {
            let (state_hash_bytes, precomputed_block_bytes) = entry?;
            let precomputed_block =
                bcs::from_bytes::<PrecomputedBlock>(precomputed_block_bytes.borrow())?;
            if let Some(length) = precomputed_block.blockchain_length {
                if length > best_tip_length {
                    best_tip_length = length;
                    best_tip_state_hash = BlockHash(bcs::from_bytes(&state_hash_bytes)?);
                }
            }
        }

        // track the best tip's parent hash back 290 blocks
        debug!("finding the transition frontier of the best tip");
        let mut root_state_hash = best_tip_state_hash.clone();
        for _i in 0..canonical_update_threshold {
            match db.get_block(&root_state_hash) {
                Ok(parent_block) => {
                    if let Some(precomputed_block) = parent_block {
                        root_state_hash = BlockHash::from_hashv1(
                            precomputed_block.protocol_state.previous_state_hash,
                        );
                    } else {
                        warn!("parent block is not in the block store");
                        break;
                    }
                }
                Err(_) => {
                    warn!("database does not have a full transition frontier");
                    break;
                }
            }
        }

        // add all blocks with chain length longer than the root to the indexer state
        // QUESTION what do we do about blocks with blockchain_length == None
        // ^^^^^^^^ currently adding them all to the state anyway, could result in dangling branches
        debug!("adding all blocks with chain length higher than computed root");
        let root_precomputed = db
            .get_block(&root_state_hash)?
            .expect("state hash from database, exists");
        let mut state = IndexerState::new_with_db(
            Block::from_precomputed(&root_precomputed, 0),
            canonical_update_threshold,
            Some(MAINNET_TRANSITION_FRONTIER_K),
            db,
        )?;
        let mut blocks_to_add = Vec::new();
        for entry in state
            .indexer_store
            .as_ref()
            .expect("guaranteed by above call")
            .iterator()
        {
            let (_state_hash_bytes, precomputed_block_bytes) = entry?;
            let precomputed_block =
                bcs::from_bytes::<PrecomputedBlock>(precomputed_block_bytes.borrow())?;
            if let Some(length) = precomputed_block.blockchain_length {
                if length > root_precomputed.blockchain_length.expect("already checked") {
                    blocks_to_add.push(precomputed_block);
                }
            } else {
                blocks_to_add.push(precomputed_block);
            }
        }
        for block in blocks_to_add {
            // QUESTION: Prune blocks that create a dangling branch?
            state.add_block(&block, false)?;
        }

        Ok(state)
    }

    /// Removes the lower portion of the root tree which is no longer needed
    fn prune_root_branch(&mut self) {
        let k = self.transition_frontier_length;
        self.update_canonical();

        if self.root_branch.height() > self.prune_interval * k {
            let best_tip_block = self.best_tip_block().clone();
            debug!(
                "Pruning transition frontier: k = {}, best tip length = {}, canonical tip length = {}",
                k,
                self.best_tip_block().blockchain_length.unwrap_or(0),
                self.canonical_tip_block().blockchain_length.unwrap_or(0),
            );

            self.root_branch
                .prune_transition_frontier(k, &best_tip_block);
        }
    }

    /// The highest known canonical block
    pub fn canonical_tip_block(&self) -> &Block {
        self.get_block_from_id(&self.canonical_tip.node_id)
    }

    /// The highest block known to be a descendant of the original root block
    pub fn best_tip_block(&self) -> &Block {
        self.get_block_from_id(&self.best_tip.node_id)
    }

    /// Only works with blocks in the root branch
    fn get_block_from_id(&self, node_id: &NodeId) -> &Block {
        self.root_branch.branches.get(node_id).unwrap().data()
    }

    /// Updates the canonical tip if the precondition is met
    pub fn update_canonical(&mut self) {
        if self.best_tip_block().height - self.canonical_tip_block().height
            > self.canonical_update_threshold
        {
            let mut canonical_hashes = vec![];
            let old_canonical_tip_id = self.canonical_tip.node_id.clone();
            let old_canonical_tip_hash = self.canonical_tip_block().state_hash.clone();

            // update canonical_tip
            for (n, ancestor_id) in self
                .root_branch
                .branches
                .ancestor_ids(&self.best_tip.node_id)
                .unwrap()
                .enumerate()
            {
                // only add blocks between the old_canonical_tip and the new one
                if n + 1 == MAINNET_CANONICAL_THRESHOLD as usize {
                    self.canonical_tip.node_id = ancestor_id.clone();
                    self.canonical_tip.state_hash =
                        self.get_block_from_id(ancestor_id).state_hash.clone();
                } else if n > MAINNET_CANONICAL_THRESHOLD as usize
                    && ancestor_id != &old_canonical_tip_id
                {
                    let ancestor_block = self.get_block_from_id(ancestor_id);
                    canonical_hashes.push(ancestor_block.state_hash.clone());
                } else if ancestor_id == &old_canonical_tip_id {
                    break;
                }
            }

            canonical_hashes.reverse();

            // update canonical ledger
            if let Some(indexer_store) = &self.indexer_store {
                let mut ledger = indexer_store
                    .get_ledger(&old_canonical_tip_hash)
                    .unwrap()
                    .unwrap();

                // apply the new canonical diffs to the old canonical ledger
                for canonical_hash in &canonical_hashes {
                    if let Some(diff) = self.diffs_map.get(canonical_hash) {
                        ledger.apply_diff(diff).unwrap();
                    }
                }

                // update the ledger
                indexer_store
                    .add_ledger(&self.canonical_tip_block().state_hash, ledger)
                    .unwrap();
            }

            // update canonicity store
            for block_hash in self.diffs_map.keys() {
                if let Some(indexer_store) = &self.indexer_store {
                    if canonical_hashes.contains(block_hash) {
                        indexer_store
                            .set_canonicity(block_hash, Canonicity::Canonical)
                            .unwrap();
                    } else {
                        indexer_store
                            .set_canonicity(block_hash, Canonicity::Orphaned)
                            .unwrap();
                    }
                }
            }

            // remove diffs corresponding to blocks at or beneath the height of the new canonical tip
            for node_id in self
                .root_branch
                .branches
                .traverse_level_order_ids(&old_canonical_tip_id)
                .unwrap()
            {
                if self.get_block_from_id(&node_id).height <= self.canonical_tip_block().height {
                    self.diffs_map
                        .remove(&self.get_block_from_id(&node_id).state_hash.clone());
                }
            }
        }
    }

    /// Initialize indexer state from a collection of contiguous canonical blocks
    pub async fn initialize_with_contiguous_canonical(
        &mut self,
        block_parser: &mut BlockParser,
    ) -> anyhow::Result<()> {
        let mut block_count = 0;
        if let Some(indexer_store) = self.indexer_store.as_ref() {
            let mut ledger = indexer_store
                .get_ledger(&self.canonical_tip.state_hash)?
                .unwrap();
            let total_time = Instant::now();

            info!("Reporting every {BLOCK_REPORTING_FREQ_NUM} blocks");
            while block_count < block_parser.num_canonical {
                block_count += 1;

                if should_report_from_block_count(block_count) {
                    let rate = block_count as f64 / total_time.elapsed().as_secs() as f64;

                    info!(
                        "{block_count} blocks parsed and applied in {:?}",
                        total_time.elapsed()
                    );
                    info!(
                        "Estimated time: {} min",
                        (block_parser.total_num_blocks - block_count) as f64 / (rate * 60_f64)
                    );
                    debug!("Rate: {rate} blocks/s");
                }

                let precomputed_block = block_parser.next().await?.unwrap();
                let diff = LedgerDiff::from_precomputed_block(&precomputed_block);

                // apply and add to db
                ledger.apply_diff(&diff)?;
                indexer_store.add_block(&precomputed_block)?;

                // TODO store ledger at specified cadence, e.g. at epoch boundaries
                // for now, just store every 1000 blocks
                if block_count % 1000 == 0 {
                    indexer_store.add_ledger(
                        &BlockHash(precomputed_block.state_hash.clone()),
                        ledger.clone(),
                    )?;
                }

                if block_count == block_parser.num_canonical {
                    // update root branch
                    self.root_branch = Branch::new(&precomputed_block)?;
                    self.best_tip = Tip {
                        state_hash: self.root_branch.root_block().state_hash.clone(),
                        node_id: self.root_branch.root.clone(),
                    };
                    self.canonical_tip = self.best_tip.clone();
                }
            }

            // store the most recent canonical ledger
            indexer_store.add_ledger(&self.root_branch.root_block().state_hash, ledger.clone())?;
        }

        // now add the successive non-canoical blocks
        self.add_blocks(block_parser, block_count).await
    }

    /// Initialize indexer state without contiguous canonical blocks
    pub async fn initialize_without_contiguous_canonical(
        &mut self,
        block_parser: &mut BlockParser,
    ) -> anyhow::Result<()> {
        self.add_blocks(block_parser, 0).await
    }

    /// Adds blocks to the state according to block_parser then changes phase to Watching
    ///
    /// Returns the number of blocks parsed
    pub async fn add_blocks(
        &mut self,
        block_parser: &mut BlockParser,
        blocks_processed: u32,
    ) -> anyhow::Result<()> {
        let mut block_count = blocks_processed;
        let total_time = Instant::now();
        let mut step_time = total_time;

        if blocks_processed == 0 {
            info!(
                "Reporting every {BLOCK_REPORTING_FREQ_SEC}s or {BLOCK_REPORTING_FREQ_NUM} blocks"
            );
        }
        while let Some(block) = block_parser.next().await? {
            if should_report_from_block_count(block_count)
                || self.should_report_from_time(step_time.elapsed())
            {
                step_time = Instant::now();

                let best_tip: BlockWithoutHeight = self.best_tip_block().clone().into();
                let canonical_tip: BlockWithoutHeight = self.canonical_tip_block().clone().into();
                let rate = block_count as f64 / total_time.elapsed().as_secs() as f64;

                info!(
                    "Parsed and added {block_count} blocks to the witness tree in {:?}",
                    total_time.elapsed()
                );

                debug!("Root height:       {}", self.root_branch.height());
                debug!("Root length:       {}", self.root_branch.len());
                debug!("Rate:              {rate} blocks/s");

                info!(
                    "Estimate rem time: {} hr",
                    (block_parser.total_num_blocks - block_count) as f64 / (rate * 3600_f64)
                );
                info!("Best tip:          {best_tip:?}");
                info!("Canonical tip:     {canonical_tip:?}");
            }

            self.add_block(&block, false)?;
            block_count += 1;
        }

        info!(
            "Ingested {block_count} blocks in {:?}",
            total_time.elapsed()
        );

        debug!("Phase change: {} -> {}", self.phase, IndexerPhase::Watching);
        self.phase = IndexerPhase::Watching;
        Ok(())
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

        if check_if_block_in_db && self.is_block_already_in_db(precomputed_block)? {
            debug!(
                "Block with state hash {:?} is already present in the block store",
                precomputed_block.state_hash
            );
            return Ok(ExtensionType::BlockNotAdded);
        }

        let incoming_length = precomputed_block.blockchain_length.unwrap_or(u32::MAX);
        if self.root_branch.root_block().blockchain_length.unwrap_or(0) > incoming_length {
            debug!(
                "Block with state hash {:?} has length {incoming_length} which is too low to add to the witness tree",
                precomputed_block.state_hash,
            );
            return Ok(ExtensionType::BlockNotAdded);
        }

        // add block to the db
        if let Some(indexer_store) = self.indexer_store.as_ref() {
            indexer_store.add_block(precomputed_block)?;
        }

        self.blocks_processed += 1;
        self.diffs_map.insert(
            BlockHash(precomputed_block.state_hash.clone()),
            LedgerDiff::from_precomputed_block(precomputed_block),
        );

        // forward extension on root branch
        if self.is_length_within_root_bounds(precomputed_block) {
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

        self.new_dangling(precomputed_block)
    }

    /// Extends the root branch forward, potentially causing dangling branches to be merged into it
    fn root_extension(
        &mut self,
        precomputed_block: &PrecomputedBlock,
    ) -> anyhow::Result<Option<ExtensionType>> {
        if let Some((new_node_id, new_block)) = self.root_branch.simple_extension(precomputed_block)
        {
            self.update_best_tip(&new_block, &new_node_id);

            // check if new block connects to a dangling branch
            let mut merged_tip_id = None;
            let mut branches_to_remove = Vec::new();

            for (index, dangling_branch) in self.dangling_branches.iter_mut().enumerate() {
                // new block is the parent of the dangling branch root
                if is_reverse_extension(dangling_branch, precomputed_block) {
                    merged_tip_id = self.root_branch.merge_on(&new_node_id, dangling_branch);
                    branches_to_remove.push(index);
                }

                same_block_added_twice(dangling_branch, precomputed_block)?;
            }

            if let Some(merged_tip_id) = merged_tip_id {
                let merged_tip_block = self
                    .root_branch
                    .branches
                    .get(&merged_tip_id)
                    .unwrap()
                    .data()
                    .clone();

                if merged_tip_block > self.best_tip_block().clone() {
                    self.update_best_tip(&merged_tip_block, &merged_tip_id);
                }
            }

            if !branches_to_remove.is_empty() {
                // the root branch is newly connected to dangling branches
                for (num_removed, index_to_remove) in branches_to_remove.iter().enumerate() {
                    self.dangling_branches.remove(index_to_remove - num_removed);
                }

                Ok(Some(ExtensionType::RootComplex))
            } else {
                // there aren't any branches that are connected
                Ok(Some(ExtensionType::RootSimple))
            }
        } else {
            Ok(None)
        }
    }

    /// Extends an existing dangling branch either forwards or backwards
    fn dangling_extension(
        &mut self,
        precomputed_block: &PrecomputedBlock,
    ) -> anyhow::Result<Option<(usize, NodeId, ExtensionDirection)>> {
        let mut extension = None;
        for (index, dangling_branch) in self.dangling_branches.iter_mut().enumerate() {
            let min_length = dangling_branch.root_block().blockchain_length.unwrap_or(0);
            let max_length = dangling_branch
                .best_tip()
                .unwrap()
                .blockchain_length
                .unwrap_or(0);

            // check incoming block is within the length bounds
            if let Some(length) = precomputed_block.blockchain_length {
                if max_length + 1 >= length && length + 1 >= min_length {
                    // simple reverse
                    if is_reverse_extension(dangling_branch, precomputed_block) {
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
                    if let Some((new_node_id, _)) =
                        dangling_branch.simple_extension(precomputed_block)
                    {
                        extension = Some((index, new_node_id, ExtensionDirection::Forward));
                        break;
                    }

                    same_block_added_twice(dangling_branch, precomputed_block)?;
                }
            } else {
                // we don't know the blockchain_length for the incoming block, so we can't discriminate

                // simple reverse
                if is_reverse_extension(dangling_branch, precomputed_block) {
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
                if let Some((new_node_id, _)) = dangling_branch.simple_extension(precomputed_block)
                {
                    extension = Some((index, new_node_id, ExtensionDirection::Forward));
                    break;
                }

                same_block_added_twice(dangling_branch, precomputed_block)?;
            }
        }

        Ok(extension)
    }

    /// Updates an existing dangling branch in the witness tree
    fn update_dangling(
        &mut self,
        precomputed_block: &PrecomputedBlock,
        extended_branch_index: usize,
        new_node_id: NodeId,
        direction: ExtensionDirection,
    ) -> anyhow::Result<ExtensionType> {
        let mut branches_to_update = Vec::new();
        for (index, dangling_branch) in self.dangling_branches.iter().enumerate() {
            if is_reverse_extension(dangling_branch, precomputed_block) {
                branches_to_update.push(index);
            }
        }

        if !branches_to_update.is_empty() {
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

    /// Spawns a new dangling branch in the witness tree
    fn new_dangling(
        &mut self,
        precomputed_block: &PrecomputedBlock,
    ) -> anyhow::Result<ExtensionType> {
        self.dangling_branches
            .push(Branch::new(precomputed_block).expect("cannot fail"));

        Ok(ExtensionType::DanglingNew)
    }

    /// Checks if it's even possible to add block to the root branch
    fn is_length_within_root_bounds(&self, precomputed_block: &PrecomputedBlock) -> bool {
        (precomputed_block.blockchain_length.is_some()
            && self.best_tip_block().blockchain_length.unwrap_or(0) + 1
                >= precomputed_block.blockchain_length.unwrap())
            || precomputed_block.blockchain_length.is_none()
    }

    fn is_block_already_in_db(&self, precomputed_block: &PrecomputedBlock) -> anyhow::Result<bool> {
        if let Some(indexer_store) = self.indexer_store.as_ref() {
            match indexer_store.get_block(&BlockHash(precomputed_block.state_hash.to_string()))? {
                None => Ok(false),
                Some(_block) => Ok(true),
            }
        } else {
            Ok(false)
        }
    }

    /// Update the best tip of the root branch
    fn update_best_tip(&mut self, incoming_block: &Block, node_id: &NodeId) {
        if let Some(incoming_length) = incoming_block.blockchain_length {
            let best_tip_length = self.best_tip_block().blockchain_length.unwrap_or(0);

            if incoming_length == best_tip_length + 1
                || incoming_length == best_tip_length && incoming_block > self.best_tip_block()
            {
                self.best_tip.node_id = node_id.clone();
                self.best_tip.state_hash = incoming_block.state_hash.clone();
            }
        }

        let (id, block) = self.root_branch.best_tip_with_id().unwrap();
        self.best_tip.node_id = id;
        self.best_tip.state_hash = block.state_hash;
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

    pub fn get_block_status(&self, state_hash: &BlockHash) -> Option<Canonicity> {
        // first check the db, then diffs map
        if let Some(indexer_store) = &self.indexer_store {
            return indexer_store.get_canonicity(state_hash).unwrap();
        } else if self.diffs_map.get(state_hash).is_some() {
            return Some(Canonicity::Pending);
        }

        None
    }

    // TODO maybe we should add another function for getting a ledger at a specific slot/"height"?
    pub fn best_ledger(&mut self) -> anyhow::Result<Option<Ledger>> {
        self.update_canonical();

        // get the most recent canonical ledger
        let ledger = if let Some(indexer_store) = &self.indexer_store {
            indexer_store.get_ledger(&self.canonical_tip_block().state_hash)?
        } else {
            None
        };

        if let Some(mut ledger) = ledger {
            // collect diffs from canonical tip to best tip
            let mut diffs_since_canonical_tip =
                if self.best_tip.state_hash != self.canonical_tip.state_hash {
                    vec![self.diffs_map.get(&self.best_tip.state_hash).unwrap()]
                } else {
                    vec![]
                };

            for ancestor in self
                .root_branch
                .branches
                .ancestors(&self.best_tip.node_id)
                .unwrap()
            {
                if ancestor.data().state_hash != self.canonical_tip.state_hash {
                    diffs_since_canonical_tip
                        .push(self.diffs_map.get(&ancestor.data().state_hash).unwrap());
                } else {
                    break;
                }
            }

            // apply diffs from canonical tip to best tip
            diffs_since_canonical_tip.reverse();
            for diff in diffs_since_canonical_tip {
                ledger.apply_diff(diff)?;
            }

            return Ok(Some(ledger));
        }

        Ok(None)
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u32 {
        let mut len = self.root_branch.len();
        for dangling in &self.dangling_branches {
            len += dangling.len();
        }
        len
    }

    pub fn summary_short(&self) -> SummaryShort {
        let mut max_dangling_height = 0;
        let mut max_dangling_length = 0;

        for dangling in &self.dangling_branches {
            if dangling.height() > max_dangling_height {
                max_dangling_height = dangling.height();
            }
            if dangling.len() > max_dangling_length {
                max_dangling_length = dangling.len();
            }
        }

        let db_stats_str = self.indexer_store.as_ref().map(|db| db.db_stats());
        let mem = self
            .indexer_store
            .as_ref()
            .map(|db| db.memtables_size())
            .unwrap_or_default();
        let witness_tree = WitnessTreeSummaryShort {
            best_tip_hash: self.best_tip_block().state_hash.0.clone(),
            best_tip_length: self.best_tip_block().blockchain_length.unwrap_or(0),
            canonical_tip_hash: self.canonical_tip_block().state_hash.0.clone(),
            canonical_tip_length: self.canonical_tip_block().blockchain_length.unwrap_or(0),
            root_hash: self.root_branch.root_block().state_hash.0.clone(),
            root_height: self.root_branch.height(),
            root_length: self.root_branch.len(),
            num_leaves: self.root_branch.leaves().len() as u32,
            num_dangling: self.dangling_branches.len() as u32,
            max_dangling_height,
            max_dangling_length,
        };

        SummaryShort {
            uptime: self.time.clone().elapsed(),
            date_time: PrimitiveDateTime::new(self.date_time.date(), self.date_time.time()),
            blocks_processed: self.blocks_processed,
            witness_tree,
            db_stats: db_stats_str.map(|s| DbStats::from_str(&format!("{mem}\n{s}")).unwrap()),
        }
    }

    pub fn summary_verbose(&self) -> SummaryVerbose {
        let mut max_dangling_height = 0;
        let mut max_dangling_length = 0;

        for dangling in &self.dangling_branches {
            if dangling.height() > max_dangling_height {
                max_dangling_height = dangling.height();
            }
            if dangling.len() > max_dangling_length {
                max_dangling_length = dangling.len();
            }
        }

        let db_stats_str = self.indexer_store.as_ref().map(|db| db.db_stats());
        let mem = self
            .indexer_store
            .as_ref()
            .map(|db| db.memtables_size())
            .unwrap_or_default();
        let witness_tree = WitnessTreeSummaryVerbose {
            best_tip_hash: self.best_tip_block().state_hash.0.clone(),
            best_tip_length: self.best_tip_block().blockchain_length.unwrap_or(0),
            canonical_tip_hash: self.canonical_tip_block().state_hash.0.clone(),
            canonical_tip_length: self.canonical_tip_block().blockchain_length.unwrap_or(0),
            root_hash: self.root_branch.root_block().state_hash.0.clone(),
            root_height: self.root_branch.height(),
            root_length: self.root_branch.len(),
            num_leaves: self.root_branch.leaves().len() as u32,
            num_dangling: self.dangling_branches.len() as u32,
            max_dangling_height,
            max_dangling_length,
            witness_tree: format!("{self:?}"),
        };

        SummaryVerbose {
            uptime: self.time.clone().elapsed(),
            date_time: PrimitiveDateTime::new(self.date_time.date(), self.date_time.time()),
            blocks_processed: self.blocks_processed,
            witness_tree,
            db_stats: db_stats_str.map(|s| DbStats::from_str(&format!("{mem}\n{s}")).unwrap()),
        }
    }

    fn is_initializing(&self) -> bool {
        self.phase == IndexerPhase::InitializingFromBlockDir
            || self.phase == IndexerPhase::InitializingFromDB
    }

    fn should_report_from_time(&self, duration: Duration) -> bool {
        self.is_initializing() && duration.as_secs() > BLOCK_REPORTING_FREQ_SEC
    }
}

/// Checks if the block is the parent of the branch's root
fn is_reverse_extension(branch: &Branch, precomputed_block: &PrecomputedBlock) -> bool {
    precomputed_block.state_hash == branch.root_block().parent_hash.0
}

/// Errors if the blocks is added a second time
fn same_block_added_twice(
    branch: &Branch,
    precomputed_block: &PrecomputedBlock,
) -> anyhow::Result<()> {
    if precomputed_block.state_hash == branch.root_block().state_hash.0 {
        return Err(anyhow::Error::msg(format!(
            "Block with hash {:?} added twice to the indexer state",
            precomputed_block.state_hash,
        )));
    }
    Ok(())
}

fn should_report_from_block_count(block_count: u32) -> bool {
    block_count > 0 && block_count % BLOCK_REPORTING_FREQ_NUM == 0
}

impl std::fmt::Debug for IndexerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Root branch ===")?;
        writeln!(f, "{:?}", self.root_branch)?;

        if !self.dangling_branches.is_empty() {
            writeln!(f, "=== Dangling branches ===")?;
            for (n, branch) in self.dangling_branches.iter().enumerate() {
                writeln!(f, "Dangling branch {n}:")?;
                writeln!(f, "{branch:?}")?;
            }
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
