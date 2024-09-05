pub mod branch;
pub mod summary;

use crate::{
    block::{
        genesis::GenesisBlock,
        parser::{BlockParser, ParsedBlock},
        precomputed::PrecomputedBlock,
        store::BlockStore,
        Block, BlockHash, BlockWithoutHeight,
    },
    canonicity::{store::CanonicityStore, Canonicity},
    chain::store::ChainStore,
    constants::*,
    event::{db::*, store::*, witness_tree::*, IndexerEvent},
    ledger::{
        diff::LedgerDiff,
        genesis::GenesisLedger,
        public_key::PublicKey,
        staking::parser::StakingLedgerParser,
        store::{staged::StagedLedgerStore, staking::StakingLedgerStore},
        username::Username,
        Ledger, LedgerHash,
    },
    server::IndexerVersion,
    state::{
        branch::Branch,
        summary::{
            DbStats, SummaryShort, SummaryVerbose, WitnessTreeSummaryShort,
            WitnessTreeSummaryVerbose,
        },
    },
    store::{
        block_state_hash_from_key, block_u32_prefix_from_key, fixed_keys::FixedKeys, from_be_bytes,
        from_u64_be_bytes, staking_ledger_store_impl::split_staking_ledger_epoch_key, to_be_bytes,
        username::UsernameStore, IndexerStore,
    },
    utility::functions::pretty_print_duration,
};
use anyhow::bail;
use id_tree::NodeId;
use log::{debug, error, info, trace};
use std::{
    collections::HashMap,
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

/// Rooted forest of precomputed block summaries aka the witness tree
/// `root_branch` - represents the tree of blocks connecting back to a known
/// ledger state, e.g. genesis `dangling_branches` - trees of blocks stemming
/// from an unknown ledger state
#[derive(Debug)]
pub struct IndexerState {
    /// Block representing the best tip of the root branch
    pub best_tip: Tip,

    /// Highest known canonical block with threshold confirmations
    pub canonical_root: Tip,

    /// Ledger corresponding to the canonical root
    pub ledger: Ledger,

    /// Cadence for computing and storing new ledgers
    pub ledger_cadence: u32,

    /// Map of ledger diffs following the canonical root
    pub diffs_map: HashMap<BlockHash, LedgerDiff>,

    /// Append-only tree of blocks built from genesis, each containing a ledger
    pub root_branch: Branch,

    /// Dynamic, dangling branches eventually merged into the `root_branch`
    /// needed for the possibility of missing blocks
    pub dangling_branches: Vec<Branch>,

    /// Underlying database
    pub indexer_store: Option<Arc<IndexerStore>>,

    /// Staking ledger epochs and ledger hashes
    pub staking_ledgers: HashMap<u32, LedgerHash>,

    /// Threshold amount of confirmations to trigger a pruning event
    pub transition_frontier_length: u32,

    /// Interval to prune the root branch
    pub prune_interval: u32,

    /// Frequency to report
    pub reporting_freq: u32,

    /// Threshold for updating the canonical root and db ledger
    pub canonical_update_threshold: u32,

    /// Threshold confirmations required to prune a canoncial block from the
    /// witness tree
    pub canonical_threshold: u32,

    /// Number of blocks added to the witness tree
    pub blocks_processed: u32,

    /// Number of block bytes added to the witness tree
    pub bytes_processed: u64,

    genesis_bytes: u64,

    /// Datetime the indexer started running
    pub init_time: Instant,

    /// Network blocks and staking ledgers to be processed
    pub version: IndexerVersion,
}

#[derive(Debug, Clone)]
pub struct Tip {
    pub state_hash: BlockHash,
    pub node_id: NodeId,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IndexerPhase {
    InitializingFromBlockDir,
    SyncingFromDB,
    Replaying,
    Watching,
    Testing,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ExtensionType {
    DanglingNew,
    DanglingSimpleForward,
    DanglingSimpleReverse,
    DanglingComplex,
    RootSimple(Block),
    RootComplex(Block),
    BlockNotAdded,
}

pub enum ExtensionDirection {
    Forward,
    Reverse,
}

pub struct IndexerStateConfig {
    pub genesis_hash: BlockHash,
    pub genesis_ledger: GenesisLedger,
    pub version: IndexerVersion,
    pub indexer_store: Arc<IndexerStore>,
    pub transition_frontier_length: u32,
    pub prune_interval: u32,
    pub canonical_threshold: u32,
    pub canonical_update_threshold: u32,
    pub ledger_cadence: u32,
    pub reporting_freq: u32,
    pub do_not_ingest_orphan_blocks: bool,
}

impl IndexerStateConfig {
    pub fn new(
        genesis_ledger: GenesisLedger,
        version: IndexerVersion,
        indexer_store: Arc<IndexerStore>,
        canonical_threshold: u32,
        transition_frontier_length: u32,
        do_not_ingest_orphan_blocks: bool,
    ) -> Self {
        IndexerStateConfig {
            version,
            genesis_ledger,
            indexer_store,
            canonical_threshold,
            transition_frontier_length,
            do_not_ingest_orphan_blocks,
            genesis_hash: MAINNET_GENESIS_HASH.into(),
            prune_interval: PRUNE_INTERVAL_DEFAULT,
            canonical_update_threshold: CANONICAL_UPDATE_THRESHOLD,
            ledger_cadence: LEDGER_CADENCE,
            reporting_freq: BLOCK_REPORTING_FREQ_NUM,
        }
    }
}

impl IndexerState {
    pub fn new(
        genesis_ledger: GenesisLedger,
        version: IndexerVersion,
        indexer_store: Arc<IndexerStore>,
        canonical_threshold: u32,
        transition_frontier_length: u32,
        do_not_ingest_orphan_blocks: bool,
    ) -> anyhow::Result<Self> {
        Self::new_from_config(IndexerStateConfig::new(
            genesis_ledger,
            version,
            indexer_store,
            canonical_threshold,
            transition_frontier_length,
            do_not_ingest_orphan_blocks,
        ))
    }

    /// Creates a new indexer state from the genesis ledger
    pub fn new_from_config(config: IndexerStateConfig) -> anyhow::Result<Self> {
        // set chain id
        config
            .indexer_store
            .set_chain_id_for_network(&config.version.chain_id, &config.version.network)?;

        let genesis_block = GenesisBlock::new()?;
        let genesis_bytes = genesis_block.1;
        let genesis_block = genesis_block.0;

        // add genesis block and ledger to indexer store
        config.indexer_store.add_genesis_ledger(
            &genesis_block.previous_state_hash(),
            config.genesis_ledger.clone().into(),
        )?;
        info!("Genesis ledger added to indexer store");

        config
            .indexer_store
            .add_block(&genesis_block, genesis_bytes)?;
        info!("Genesis block added to indexer store");

        // update genesis canonicity
        config.indexer_store.add_canonical_block(
            1,
            0,
            &genesis_block.state_hash(),
            &genesis_block.state_hash(),
            Some(&genesis_block.previous_state_hash()),
        )?;

        // update genesis best block
        config
            .indexer_store
            .set_best_block(&genesis_block.state_hash())?;

        // apply genesis block to genesis ledger and keep its ledger diff
        let root_branch = Branch::new_genesis(
            genesis_block.state_hash(),
            genesis_block.previous_state_hash(),
        )?;
        let tip = Tip {
            state_hash: root_branch.root_block().state_hash.clone(),
            node_id: root_branch.root.clone(),
        };
        Ok(Self {
            ledger: <GenesisLedger as Into<Ledger>>::into(config.genesis_ledger)
                .apply_diff_from_precomputed(&genesis_block)?,
            diffs_map: HashMap::from([(
                genesis_block.state_hash(),
                LedgerDiff::from_precomputed(&genesis_block),
            )]),
            canonical_root: tip.clone(),
            best_tip: tip,
            root_branch,
            version: config.version,
            dangling_branches: Vec::new(),
            indexer_store: Some(config.indexer_store),
            transition_frontier_length: config.transition_frontier_length,
            prune_interval: config.prune_interval,
            canonical_threshold: config.canonical_threshold,
            canonical_update_threshold: config.canonical_update_threshold,
            blocks_processed: 1, // genesis block
            bytes_processed: genesis_bytes,
            genesis_bytes,
            init_time: Instant::now(),
            ledger_cadence: config.ledger_cadence,
            reporting_freq: config.reporting_freq,
            staking_ledgers: HashMap::new(),
        })
    }

    /// Creates a new indexer state without genesis events
    pub fn new_without_genesis_events(config: IndexerStateConfig) -> anyhow::Result<Self> {
        let root_branch =
            Branch::new_genesis(config.genesis_hash, MAINNET_GENESIS_PREV_STATE_HASH.into())?;
        let tip = Tip {
            state_hash: root_branch.root_block().state_hash.clone(),
            node_id: root_branch.root.clone(),
        };

        Ok(Self {
            ledger: config.genesis_ledger.into(),
            diffs_map: HashMap::new(),
            canonical_root: tip.clone(),
            best_tip: tip,
            root_branch,
            version: config.version,
            dangling_branches: Vec::new(),
            indexer_store: Some(config.indexer_store),
            transition_frontier_length: config.transition_frontier_length,
            prune_interval: config.prune_interval,
            canonical_threshold: config.canonical_threshold,
            canonical_update_threshold: config.canonical_update_threshold,
            blocks_processed: 0, // no genesis block included
            genesis_bytes: 0,
            bytes_processed: 0,
            init_time: Instant::now(),
            ledger_cadence: config.ledger_cadence,
            reporting_freq: config.reporting_freq,
            staking_ledgers: HashMap::new(),
        })
    }

    /// Creates a new indexer state for testing
    pub fn new_testing(
        root_block: &PrecomputedBlock,
        root_block_bytes: u64,
        root_ledger: Option<Ledger>,
        speedb_path: Option<&std::path::Path>,
        transition_frontier_length: Option<u32>,
        ledger_cadence: Option<u32>,
        reporting_freq: Option<u32>,
    ) -> anyhow::Result<Self> {
        let root_branch = Branch::new_testing(root_block);
        let indexer_store = speedb_path.map(|path| {
            let store = IndexerStore::new(path).unwrap();
            if let Some(ledger) = root_ledger.clone() {
                store
                    .add_staged_ledger_at_state_hash(&root_block.state_hash(), ledger)
                    .expect("ledger add succeeds");
                store
                    .set_best_block(&root_block.state_hash())
                    .expect("set best block to root block");
            }
            store
        });

        let tip = Tip {
            state_hash: root_branch.root_block().state_hash.clone(),
            node_id: root_branch.root.clone(),
        };

        // apply root block to root ledger and keep its ledger diff
        Ok(Self {
            ledger: root_ledger
                .and_then(|x| x.apply_diff_from_precomputed(root_block).ok())
                .unwrap_or_default(),
            diffs_map: HashMap::from([(
                root_block.state_hash(),
                LedgerDiff::from_precomputed(root_block),
            )]),
            canonical_root: tip.clone(),
            best_tip: tip,
            root_branch,
            dangling_branches: Vec::new(),
            indexer_store: indexer_store.map(Arc::new),
            transition_frontier_length: transition_frontier_length
                .unwrap_or(MAINNET_TRANSITION_FRONTIER_K),
            prune_interval: PRUNE_INTERVAL_DEFAULT,
            canonical_threshold: MAINNET_CANONICAL_THRESHOLD,
            canonical_update_threshold: CANONICAL_UPDATE_THRESHOLD,
            blocks_processed: 1, // root block
            bytes_processed: root_block_bytes,
            genesis_bytes: root_block_bytes,
            init_time: Instant::now(),
            ledger_cadence: ledger_cadence.unwrap_or(LEDGER_CADENCE),
            reporting_freq: reporting_freq.unwrap_or(BLOCK_REPORTING_FREQ_NUM),
            staking_ledgers: HashMap::new(),
            version: IndexerVersion::default(),
        })
    }

    /// Initialize indexer state from a collection of contiguous canonical
    /// blocks
    ///
    /// Short-circuits adding canonical blocks to the witness tree
    pub async fn initialize_with_canonical_chain_discovery(
        &mut self,
        block_parser: &mut BlockParser,
    ) -> anyhow::Result<()> {
        info!("Initializing indexer with canonical chain blocks");
        let total_time = Instant::now();
        if let Some(indexer_store) = self.indexer_store.as_ref() {
            let mut ledger_diffs = vec![];

            if block_parser.num_deep_canonical_blocks > self.reporting_freq {
                info!(
                    "Adding blocks to the witness tree, reporting every {} ...",
                    self.reporting_freq
                );
            } else {
                info!("Adding blocks to the witness tree...");
            }

            let pk_of_interest =
                &PublicKey("B62qjHdYUPTHQkwDWUbDYscteT2LFj3ro1vz9fnxMyHTACe6C2fLbSd".to_string());

            // process deep canonical blocks first bypassing the witness tree
            while self.blocks_processed <= block_parser.num_deep_canonical_blocks {
                self.blocks_processed += 1;
                self.report_from_block_count(block_parser, total_time);

                if let Some((ParsedBlock::DeepCanonical(block), block_bytes)) =
                    block_parser.next_block().await?
                {
                    let state_hash = block.state_hash();
                    self.bytes_processed += block_bytes;

                    // apply diff + add to db
                    let diff = LedgerDiff::from_precomputed(&block);
                    if diff.public_keys_seen.contains(pk_of_interest) {
                        println!("{:?}", diff);
                    }
                    ledger_diffs.push(diff.clone());

                    indexer_store.add_block(&block, block_bytes)?;
                    indexer_store.set_best_block(&block.state_hash())?;
                    indexer_store.add_canonical_block(
                        block.blockchain_length(),
                        block.global_slot_since_genesis(),
                        &block.state_hash(),
                        &block.genesis_state_hash(),
                        None,
                    )?;

                    // compute and store ledger at specified cadence
                    if self.blocks_processed % self.ledger_cadence == 0 {
                        for diff in ledger_diffs.iter() {
                            self.ledger._apply_diff(diff)?;
                        }

                        ledger_diffs.clear();
                        indexer_store
                            .add_staged_ledger_at_state_hash(&state_hash, self.ledger.clone())?;
                    }

                    // update root branch on last deep canonical block
                    if self.blocks_processed > block_parser.num_deep_canonical_blocks {
                        self.root_branch = Branch::new(&block)?;
                        self.ledger._apply_diff(&diff)?;
                        self.best_tip = Tip {
                            state_hash: self.root_branch.root_block().state_hash.clone(),
                            node_id: self.root_branch.root.clone(),
                        };
                        self.canonical_root = self.best_tip.clone();
                    }
                } else {
                    bail!("Block unexpectedly missing");
                }
            }

            assert_eq!(
                self.blocks_processed,
                block_parser.num_deep_canonical_blocks + 1
            ); // +1 genesis
        }

        self.report_from_block_count(block_parser, total_time);
        info!("Finished processing deep canonical chain");
        info!("Adding recent blocks to the witness tree and orphaned blocks to the block store");

        // deep canonical & recent blocks added, now add orphaned blocks
        self.add_blocks_with_time(block_parser, Some(total_time))
            .await
    }

    /// Adds blocks to the state according to `block_parser` then changes phase
    /// to Watching
    pub async fn add_blocks(&mut self, block_parser: &mut BlockParser) -> anyhow::Result<()> {
        self.add_blocks_with_time(block_parser, None).await
    }

    async fn add_blocks_with_time(
        &mut self,
        block_parser: &mut BlockParser,
        start: Option<Instant>,
    ) -> anyhow::Result<()> {
        let total_time = start.unwrap_or(Instant::now());
        let offset = total_time.elapsed();
        let mut step_time = total_time;

        if block_parser.total_num_blocks > self.reporting_freq {
            info!(
                "Reporting every {BLOCK_REPORTING_FREQ_SEC}s or {} blocks",
                self.reporting_freq
            );
        }

        loop {
            tokio::select! {
                // wait for SIGINT
                _ = tokio::signal::ctrl_c() => {
                    info!("SIGINT received");
                    break;
                }

                // parse the next precomputed block
                res = block_parser.next_block() => {
                    match res {
                        Ok(Some((parsed_block, block_bytes))) => {
                            self.report_progress(block_parser, step_time, total_time)?;
                            step_time = Instant::now();

                            match parsed_block {
                                ParsedBlock::DeepCanonical(block) | ParsedBlock::Recent(block) => {
                                    info!("Adding block to witness tree {}", block.summary());
                                    self.block_pipeline(&block, block_bytes)?;
                                }
                                ParsedBlock::Orphaned(block) => {
                                    trace!("Adding orphaned block to store {}", block.summary());
                                    self.add_block_to_store(&block, block_bytes, true)?;
                                }
                            }
                        }
                        Ok(None) => {
                            info!(
                                "Finished ingesting and applying {} blocks ({}) to the witness tree in {}",
                                self.blocks_processed,
                                bytesize::ByteSize::b(self.bytes_processed),
                                pretty_print_duration(total_time.elapsed() + offset),
                            );
                            break;
                        }
                        Err(e) => {
                            error!("Block ingestion error: {e}");
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// **Block pipeline**
    /// - add block to
    ///     - block store
    ///     - witness tree
    /// - db processes
    ///     - best block update
    ///     - new deep canonical blocks
    pub fn block_pipeline(
        &mut self,
        block: &PrecomputedBlock,
        block_bytes: u64,
    ) -> anyhow::Result<bool> {
        if let Some(db_event) = self.add_block_to_store(block, block_bytes, false)? {
            self.bytes_processed += block_bytes;
            let (best_tip, new_canonical_blocks) = if db_event.is_new_block_event() {
                if let Some(wt_event) = self.add_block_to_witness_tree(block, true)?.1 {
                    match wt_event {
                        WitnessTreeEvent::UpdateBestTip {
                            best_tip,
                            canonical_blocks,
                        } => (best_tip, canonical_blocks),
                    }
                } else {
                    return Ok(true);
                }
            } else {
                debug!("Block not added: {db_event:?}");
                return Ok(false);
            };

            if let Some(username_updates) = self.update_best_block_in_store(&best_tip.state_hash)? {
                for (pk, username) in username_updates.iter() {
                    if let Some(account) = self.ledger.accounts.get_mut(pk) {
                        account.username = Some(username.clone());
                    }
                }
            }
            new_canonical_blocks.iter().for_each(|block| {
                self.add_canonical_block_to_store(block, &block.genesis_state_hash, None)
                    .unwrap()
            });
        }

        Ok(true)
    }

    /// Adds the block to the witness tree & skips store operations
    pub fn add_block_to_witness_tree(
        &mut self,
        precomputed_block: &PrecomputedBlock,
        incremnet_blocks: bool,
    ) -> anyhow::Result<(ExtensionType, Option<WitnessTreeEvent>)> {
        let incoming_length = precomputed_block.blockchain_length();
        if self.root_branch.root_block().blockchain_length > incoming_length {
            error!(
                "Block {} is too low to be added to the witness tree",
                precomputed_block.summary()
            );
            return Ok((ExtensionType::BlockNotAdded, None));
        }

        // put the pcb's ledger diff in the map
        self.diffs_map.insert(
            precomputed_block.state_hash(),
            LedgerDiff::from_precomputed(precomputed_block),
        );
        if incremnet_blocks {
            self.blocks_processed += 1;
        }

        // forward extension on root branch
        if self.is_length_within_root_bounds(precomputed_block) {
            if let Some(root_extension) = self.root_extension(precomputed_block)? {
                let best_tip = match &root_extension {
                    ExtensionType::RootSimple(block) => block.clone(),
                    ExtensionType::RootComplex(block) => block.clone(),
                    _ => unreachable!(),
                };
                return Ok((
                    root_extension,
                    Some(WitnessTreeEvent::UpdateBestTip {
                        best_tip,
                        canonical_blocks: self.prune_root_branch()?,
                    }),
                ));
            }
        }

        // if a dangling branch has been extended (forward or reverse) check for new
        // connections to other dangling branches
        if let Some((extended_branch_index, new_node_id, direction)) =
            self.dangling_extension(precomputed_block)?
        {
            return self
                .update_dangling(
                    precomputed_block,
                    extended_branch_index,
                    new_node_id,
                    direction,
                )
                .map(|ext| (ext, None));
        }

        self.new_dangling(precomputed_block).map(|ext| (ext, None))
    }

    /// Extends the root branch forward, potentially causing dangling branches
    /// to be merged into it
    fn root_extension(
        &mut self,
        precomputed_block: &PrecomputedBlock,
    ) -> anyhow::Result<Option<ExtensionType>> {
        if let Some((new_node_id, new_block)) = self.root_branch.simple_extension(precomputed_block)
        {
            trace!("Root extension block {}", precomputed_block.summary());
            // check if new block connects to a dangling branch
            let mut merged_tip_ids = vec![];
            let mut branches_to_remove = Vec::new();

            for (index, dangling_branch) in self.dangling_branches.iter_mut().enumerate() {
                // new block is the parent of the dangling branch root
                if is_reverse_extension(dangling_branch, precomputed_block) {
                    merged_tip_ids.push(
                        self.root_branch
                            .merge_on(&new_node_id, dangling_branch)
                            .unwrap(),
                    );
                    branches_to_remove.push(index);
                }
            }

            let best_tip_id = merged_tip_ids.iter().min_by(|a, b| {
                let a_best_block = self.root_branch.branches.get(a).unwrap().data().clone();
                let b_best_block = self.root_branch.branches.get(b).unwrap().data().clone();
                a_best_block.cmp(&b_best_block)
            });
            if let Some(merged_tip_id) = best_tip_id {
                let merged_tip_block = self
                    .root_branch
                    .branches
                    .get(merged_tip_id)
                    .unwrap()
                    .data()
                    .clone();
                self.update_best_tip(&merged_tip_block, merged_tip_id);
            }
            self.update_best_tip(&new_block, &new_node_id);

            if !branches_to_remove.is_empty() {
                // the root branch is newly connected to dangling branches
                for (num_removed, index_to_remove) in branches_to_remove.iter().enumerate() {
                    self.dangling_branches.remove(index_to_remove - num_removed);
                }
                Ok(Some(ExtensionType::RootComplex(
                    self.best_tip_block().clone(),
                )))
            } else {
                // there aren't any branches that are connected
                Ok(Some(ExtensionType::RootSimple(
                    self.best_tip_block().clone(),
                )))
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
            let min_length = dangling_branch.root_block().blockchain_length;
            let max_length = dangling_branch.best_tip().unwrap().blockchain_length;

            // check incoming block is within the length bounds
            if max_length + 1 >= precomputed_block.blockchain_length()
                && precomputed_block.blockchain_length() + 1 >= min_length
            {
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
        self.dangling_branches.push(Branch::new(precomputed_block)?);
        Ok(ExtensionType::DanglingNew)
    }

    /// Checks if it's even possible to add block to the root branch
    fn is_length_within_root_bounds(&self, precomputed_block: &PrecomputedBlock) -> bool {
        self.best_tip_block().blockchain_length + 1 >= precomputed_block.blockchain_length()
    }

    /// Update the best tip of the root branch if the incoming block is better
    fn update_best_tip(&mut self, incoming_block: &Block, node_id: &NodeId) {
        let old_best_tip = self.best_tip_block();
        if incoming_block < old_best_tip {
            info!(
                "Update best tip\n    old: {}\n    new: {}",
                old_best_tip.summary(),
                incoming_block.summary(),
            );
            self.best_tip.node_id = node_id.clone();
            self.best_tip.state_hash = incoming_block.state_hash.clone();
        } else {
            debug!("Best block is better than the incoming block");
        }
    }

    /// Removes the lower portion of the root tree which is no longer needed
    fn prune_root_branch(&mut self) -> anyhow::Result<Vec<Block>> {
        let k = self.transition_frontier_length;
        let canonical_event = self.update_canonical()?;
        if self.root_branch.height() > self.prune_interval * k {
            let best_tip_block = self.best_tip_block().clone();
            debug!(
                "Pruning transition frontier: k = {}, best tip length = {}, canonical root length = {}",
                k,
                self.best_tip_block().blockchain_length,
                self.canonical_root_block().blockchain_length,
            );

            self.root_branch
                .prune_transition_frontier(k, &best_tip_block);
        }
        Ok(canonical_event)
    }

    /// The highest known canonical block
    pub fn canonical_root_block(&self) -> &Block {
        self.get_block_from_id(&self.canonical_root.node_id)
    }

    /// The highest block known to be a descendant of the root block
    pub fn best_tip_block(&self) -> &Block {
        self.get_block_from_id(&self.best_tip.node_id)
    }

    /// Only works with blocks in the root branch
    fn get_block_from_id(&self, node_id: &NodeId) -> &Block {
        self.root_branch.branches.get(node_id).unwrap().data()
    }

    /// Updates the canonical root if the precondition is met
    pub fn update_canonical(&mut self) -> anyhow::Result<Vec<Block>> {
        if self.is_canonical_updatable() {
            let old_canonical_root_id = self.canonical_root.node_id.clone();
            let new_canonical_blocks = self.get_new_canonical_blocks(&old_canonical_root_id)?;

            self.update_ledger(&new_canonical_blocks)?;
            self.update_ledger_store(&new_canonical_blocks)?;
            self.prune_diffs_map(&old_canonical_root_id)?;

            return Ok(new_canonical_blocks);
        }
        Ok(vec![])
    }

    fn is_canonical_updatable(&self) -> bool {
        self.best_tip_block().height - self.canonical_root_block().height
            >= self.canonical_update_threshold
    }

    /// Get the status of a block: Canonical, Pending, or Orphaned
    pub fn get_block_status(&self, state_hash: &BlockHash) -> anyhow::Result<Option<Canonicity>> {
        if let Some(indexer_store) = self.indexer_store.as_ref() {
            return indexer_store.get_block_canonicity(state_hash);
        }
        Ok(None)
    }

    /// Returns the best chain back to the root of the witness tree
    pub fn best_chain(&self) -> Vec<Block> {
        let mut best_chain = vec![self.best_tip_block().clone()];
        for b in self
            .root_branch
            .branches
            .ancestors(&self.best_tip.node_id)
            .unwrap()
        {
            best_chain.push(b.data().clone());
            if b.data() == self.canonical_root_block() {
                break;
            }
        }
        best_chain
    }

    /// Get the canonical block at the given height
    pub fn canonical_block_at_height(
        &self,
        height: u32,
    ) -> anyhow::Result<Option<PrecomputedBlock>> {
        if let Some(indexer_store) = self.indexer_store.as_ref() {
            if let Ok(Some(state_hash)) = indexer_store.get_canonical_hash_at_height(height) {
                return Ok(indexer_store.get_block(&state_hash)?.map(|b| b.0));
            }
        }
        Ok(None)
    }

    pub fn len(&self) -> u32 {
        let mut len = self.root_branch.len();
        for dangling in &self.dangling_branches {
            len += dangling.len();
        }
        len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Add staking ledgers to the underlying ledger store
    pub async fn add_startup_staking_ledgers_to_store(
        &mut self,
        ledgers_dir: &std::path::Path,
    ) -> anyhow::Result<()> {
        match std::fs::read_dir(ledgers_dir) {
            Ok(dir) => {
                if dir.count() > 0 {
                    info!("Parsing staking ledgers in {}", ledgers_dir.display());
                }
            }
            Err(e) => error!("Error reading staking ledgers: {e}"),
        }

        // parse staking ledgers in ledgers_dir if not it db already
        let mut ledger_parser = StakingLedgerParser::new(ledgers_dir)?;
        if let Some(indexer_store) = self.indexer_store.as_ref() {
            loop {
                tokio::select! {
                    // wait for SIGINT
                    _ = tokio::signal::ctrl_c() => {
                        info!("SIGINT received");
                        break;
                    }

                    // parse the next staking ledger
                    res = ledger_parser.next_ledger(self.indexer_store.as_ref()) => {
                        match res {
                            Ok(Some(staking_ledger)) => {
                                let summary = staking_ledger.summary();
                                self.staking_ledgers
                                    .insert(staking_ledger.epoch, staking_ledger.ledger_hash.clone());
                                indexer_store
                                    .add_staking_ledger(staking_ledger, &self.version.genesis_state_hash)?;
                                info!("Added staking ledger {summary}");
                            }
                            Ok(None) => {
                                break;
                            }
                            Err(e) => {
                                panic!("Staking ledger ingestion error: {e}");
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Add block to the underlying block store
    pub fn add_block_to_store(
        &mut self,
        block: &PrecomputedBlock,
        num_block_bytes: u64,
        increment_blocks: bool,
    ) -> anyhow::Result<Option<DbEvent>> {
        if increment_blocks {
            self.blocks_processed += 1;
            self.bytes_processed += num_block_bytes;
        }
        if let Some(indexer_store) = self.indexer_store.as_ref() {
            return indexer_store.add_block(block, num_block_bytes);
        }
        Ok(None)
    }

    fn add_canonical_block_to_store(
        &self,
        block: &Block,
        genesis_state_hash: &BlockHash,
        genesis_prev_state_hash: Option<&BlockHash>,
    ) -> anyhow::Result<()> {
        if let Some(indexer_store) = self.indexer_store.as_ref() {
            indexer_store.add_canonical_block(
                block.blockchain_length,
                block.global_slot_since_genesis,
                &block.state_hash,
                genesis_state_hash,
                genesis_prev_state_hash,
            )?;
        }
        Ok(())
    }

    pub fn update_best_block_in_store(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<HashMap<PublicKey, Username>>> {
        if let Some(indexer_store) = self.indexer_store.as_ref() {
            indexer_store.set_best_block(state_hash)?;
            return indexer_store.get_block_username_updates(state_hash);
        }
        Ok(None)
    }

    /// Sync from an existing db
    ///
    /// Short-circuits adding all blocks to the witness tree by rooting the
    /// witness tree `canonical_threshold` blocks behind the current best tip
    pub fn sync_from_db(&mut self) -> anyhow::Result<Option<u32>> {
        let mut min_length_filter = None;
        let mut witness_tree_blocks = vec![];
        let mut staking_ledgers = HashMap::new();
        if let Some(indexer_store) = self.indexer_store.as_ref() {
            debug!("Looking for witness tree root block");
            let next_seq_num = indexer_store.get_next_seq_num()?;
            let best_block_height = indexer_store.get_best_block_height()?.unwrap_or_default();
            let witness_tree_root_block_event = indexer_store
                .event_log_iterator(speedb::IteratorMode::From(
                    &next_seq_num.to_be_bytes(),
                    speedb::Direction::Reverse,
                ))
                .flatten()
                .find_map(|(_, bytes)| {
                    // value prefix == best block height or 0 BE bytes
                    let height = from_be_bytes(bytes[..4].to_vec());
                    if bytes[4] == IndexerEvent::NEW_BEST_TIP_KIND
                        && height
                            == 1.max(best_block_height.saturating_sub(self.canonical_threshold))
                    {
                        return serde_json::from_slice::<IndexerEvent>(&bytes[5..]).ok();
                    }
                    None
                });

            if let Some(IndexerEvent::Db(DbEvent::Block(DbBlockEvent::NewBestTip {
                state_hash,
                blockchain_length: root_block_height,
                ..
            }))) = witness_tree_root_block_event.as_ref()
            {
                // Get witness tree root branch root block & add all successive new blocks
                if let Some((root_block, _)) = indexer_store.get_block(state_hash)? {
                    self.root_branch = Branch::new(&root_block)?;

                    let tip = Tip {
                        state_hash: self.root_branch.root_block().state_hash.clone(),
                        node_id: self.root_branch.root.clone(),
                    };
                    self.diffs_map.insert(
                        tip.state_hash.clone(),
                        LedgerDiff::from_precomputed(&root_block),
                    );
                    self.canonical_root = tip.clone();
                    self.best_tip = tip;
                    debug!("Witness tree root block (length {root_block_height}): {state_hash}");

                    // collect witness tree blocks
                    indexer_store
                        .blocks_height_iterator(speedb::IteratorMode::From(
                            &to_be_bytes(root_block.blockchain_length()),
                            speedb::Direction::Forward,
                        ))
                        .flatten()
                        .for_each(|(key, _)| {
                            if let (Ok(height), Ok(state_hash)) = (block_u32_prefix_from_key(&key), block_state_hash_from_key(&key)) {
                                if let Ok(Some((block, _))) = indexer_store.get_block(&state_hash) {
                                    if height > 1 {
                                        witness_tree_blocks.push(block);
                                    }
                                } else {
                                    panic!(
                                        "Fatal sync error: block missing from db (length {height}): {state_hash}"
                                    )
                                }
                            }
                        });

                    // collect staking ledger data
                    for (key, _) in indexer_store
                        .staking_ledger_epoch_iterator(speedb::IteratorMode::End)
                        .flatten()
                    {
                        if let Some((genesis_state_hash, epoch, ledger_hash)) =
                            split_staking_ledger_epoch_key(&key)
                        {
                            if genesis_state_hash.0 == MAINNET_GENESIS_HASH {
                                staking_ledgers.insert(epoch, ledger_hash);
                            } else {
                                error!("Unrecognized genesis state hash");
                            }
                        }
                    }
                } else {
                    panic!("Fatal sync error: block missing from db {state_hash}")
                }

                // return after adding succesive blocks
                min_length_filter = Some(*root_block_height);
            } else {
                // add all blocks to the witness tree
                indexer_store
                    .blocks_height_iterator(speedb::IteratorMode::From(&to_be_bytes(1), speedb::Direction::Reverse))
                    .flatten()
                    .for_each(|(key, _)| {
                        if let (Ok(height), Ok(state_hash)) = (block_u32_prefix_from_key(&key), block_state_hash_from_key(&key)) {
                            if let Ok(Some((block, _))) = indexer_store.get_block(&state_hash) {
                                witness_tree_blocks.push(block);
                            } else {
                                panic!(
                                    "Fatal sync error: block missing from db (length {height}): {state_hash}"
                                )
                            }
                        }
                    });
            }
            self.blocks_processed = indexer_store.get_block_production_total_count()?;
            self.bytes_processed = indexer_store
                .database
                .get(IndexerStore::NUM_BLOCK_BYTES_PROCESSED)?
                .map_or(self.genesis_bytes, from_u64_be_bytes);
        } else {
            panic!("Fatal sync error: no indexer store");
        };

        // update witness tree blocks/staking ledgers
        self.staking_ledgers = staking_ledgers;
        for block in witness_tree_blocks {
            debug!("Sync: add block {}", block.summary());
            self.add_block_to_witness_tree(&block, false)?;
        }
        Ok(min_length_filter)
    }

    /// Replay events on a mutable state
    pub fn replay_events(&mut self, state: &Self) -> anyhow::Result<Option<u32>> {
        let mut min_length_filter = None;
        if let Some(indexer_store) = state.indexer_store.as_ref() {
            indexer_store
                .event_log_iterator(speedb::IteratorMode::Start)
                .flatten()
                .for_each(|(_, bytes)| {
                    if let Ok(ref event) = serde_json::from_slice(&bytes[5..]) {
                        if let IndexerEvent::Db(DbEvent::Canonicity(
                            DbCanonicityEvent::NewCanonicalBlock {
                                blockchain_length, ..
                            },
                        )) = event
                        {
                            // filter out blocks at or below the witness tree root
                            if Some(*blockchain_length) > min_length_filter {
                                min_length_filter = Some(*blockchain_length)
                            }
                        }
                        self.replay_event(event).unwrap_or_else(|e| error!("{e}"));
                    }
                });
        }
        Ok(min_length_filter)
    }

    fn replay_event(&mut self, event: &IndexerEvent) -> anyhow::Result<()> {
        match event {
            IndexerEvent::Db(db_event) => match db_event {
                DbEvent::Block(db_block_event) => match db_block_event {
                    DbBlockEvent::NewBestTip {
                        state_hash,
                        blockchain_length,
                    } => {
                        let indexer_store = self.indexer_store_or_panic();
                        let block_summary = format!("(length {blockchain_length}): {state_hash}");
                        info!("Replaying new best tip {block_summary}");

                        if let Some((block, _)) = indexer_store.get_block(state_hash)? {
                            assert_eq!(block.state_hash(), *state_hash);
                            assert_eq!(block.blockchain_length(), *blockchain_length);
                            assert_eq!(
                                indexer_store.get_block_height(state_hash)?,
                                Some(*blockchain_length),
                            );
                            return Ok(());
                        }
                        panic!("Fatal: block not in store {block_summary}");
                    }
                    DbBlockEvent::NewBlock {
                        blockchain_length,
                        state_hash,
                    } => {
                        // add block to the witness tree
                        let indexer_store = self.indexer_store_or_panic();
                        let block_summary = format!("(length {blockchain_length}): {state_hash}");
                        info!("Replaying db new block {block_summary}");

                        if let Ok(Some((block, _))) = indexer_store.get_block(state_hash) {
                            assert_eq!(block.state_hash(), *state_hash);
                            assert_eq!(block.blockchain_length(), *blockchain_length);
                            assert_eq!(
                                indexer_store.get_block_height(state_hash)?,
                                Some(*blockchain_length),
                            );
                            self.add_block_to_witness_tree(&block, true)?;
                            return Ok(());
                        }
                        panic!("Fatal: block missing from store {block_summary}")
                    }
                },
                DbEvent::Ledger(DbLedgerEvent::NewLedger {
                    state_hash,
                    blockchain_length,
                    ledger_hash,
                }) => {
                    let block_summary = format!("(length {blockchain_length}): {state_hash}");
                    info!("Replaying new staged ledger {ledger_hash} {block_summary}");

                    // check ledger & block are in the store
                    let indexer_store = self.indexer_store_or_panic();
                    if let Some(_ledger) =
                        indexer_store.get_staged_ledger_at_state_hash(state_hash, false)?
                    {
                        if let Some((block, _)) = indexer_store.get_block(state_hash)? {
                            assert_eq!(block.state_hash(), *state_hash);
                            return Ok(());
                        }
                        if state_hash.0 == MAINNET_GENESIS_PREV_STATE_HASH {
                            return Ok(());
                        } else {
                            panic!("Fatal: block missing from store {block_summary}");
                        }
                    }
                    panic!(
                        "Fatal: staged ledger missing from store {ledger_hash} for block {block_summary}",
                    );
                }
                DbEvent::StakingLedger(DbStakingLedgerEvent::NewStakingLedger {
                    epoch,
                    genesis_state_hash: _,
                    ledger_hash,
                }) => {
                    self.staking_ledgers.insert(*epoch, ledger_hash.clone());
                    self.replay_staking_ledger(epoch, ledger_hash)
                }
                DbEvent::StakingLedger(DbStakingLedgerEvent::AggregateDelegations {
                    epoch,
                    genesis_state_hash,
                }) => {
                    info!("Replaying aggregate delegations epoch {epoch}");
                    let genesis_state_hash = Some(genesis_state_hash.clone());
                    let indexer_store = self.indexer_store_or_panic();
                    if let Some(aggregated_delegations) =
                        indexer_store.build_aggregated_delegations(*epoch, &genesis_state_hash)?
                    {
                        if let Some(staking_ledger) =
                            indexer_store.build_staking_ledger(*epoch, &genesis_state_hash)?
                        {
                            // check delegation calculations
                            assert_eq!(
                                aggregated_delegations,
                                staking_ledger.aggregate_delegations()?
                            );
                            return Ok(());
                        }
                        panic!("Fatal: no staking ledger epoch {epoch}");
                    }
                    panic!("Fatal: aggregate delegations epoch {epoch}");
                }
                DbEvent::Canonicity(DbCanonicityEvent::NewCanonicalBlock {
                    state_hash,
                    blockchain_length,
                }) => {
                    let indexer_store = self.indexer_store_or_panic();
                    let block_summary = format!("(length {blockchain_length}): {state_hash}");
                    info!("Replay new canonical block {block_summary}");

                    // check canonicity & block store
                    if let Some(canonical_hash) =
                        indexer_store.get_canonical_hash_at_height(*blockchain_length)?
                    {
                        assert_eq!(canonical_hash, *state_hash);
                        if let Some((block, _)) = indexer_store.get_block(state_hash)? {
                            assert_eq!(block.state_hash(), *state_hash);
                            assert_eq!(block.blockchain_length(), *blockchain_length);
                            assert_eq!(
                                indexer_store.get_block_height(state_hash)?,
                                Some(*blockchain_length),
                            );
                            return Ok(());
                        }
                        panic!("Fatal: block not in store {block_summary}");
                    }
                    panic!("Fatal: canonical block not in store {block_summary}");
                }
            },
            IndexerEvent::WitnessTree(_) => unreachable!("Replay witness tree event"),
        }
    }

    fn replay_staking_ledger(&self, epoch: &u32, ledger_hash: &LedgerHash) -> anyhow::Result<()> {
        let ledger_summary = format!("(epoch {epoch}): {ledger_hash}");
        info!("Replaying staking ledger {ledger_summary}");

        // check ledger at hash & epoch
        let indexer_store = self.indexer_store_or_panic();
        if let Some(staking_ledger_hash) =
            indexer_store.get_staking_ledger(ledger_hash, Some(*epoch), &None)?
        {
            assert_eq!(staking_ledger_hash.epoch, *epoch, "Invalid epoch");
            assert_eq!(
                staking_ledger_hash.ledger_hash, *ledger_hash,
                "Invalid ledger hash"
            );

            if let Some(staking_ledger_epoch) = indexer_store.build_staking_ledger(
                *epoch,
                &Some(staking_ledger_hash.genesis_state_hash.clone()),
            )? {
                assert_eq!(staking_ledger_epoch.epoch, *epoch, "Invalid epoch");
                assert_eq!(
                    staking_ledger_epoch.ledger_hash, *ledger_hash,
                    "Invalid ledger hash"
                );

                // check ledgers are equal
                assert_eq!(staking_ledger_epoch, staking_ledger_hash);
                return Ok(());
            }
            panic!("Fatal: no staking ledger at epoch {epoch} in store");
        }
        panic!("Fatal: no staking ledger with hash {ledger_hash} in store");
    }

    fn indexer_store_or_panic(&self) -> &Arc<IndexerStore> {
        match self.indexer_store.as_ref() {
            Some(store) => store,
            None => panic!("Fatal: indexer store missing"),
        }
    }

    fn get_new_canonical_blocks(
        &mut self,
        old_canonical_root_id: &NodeId,
    ) -> anyhow::Result<Vec<Block>> {
        let mut canonical_blocks = vec![];

        for ancestor_id in self
            .root_branch
            .branches
            .ancestor_ids(&self.best_tip.node_id)
            .unwrap()
            .skip(MAINNET_CANONICAL_THRESHOLD.saturating_sub(1) as usize)
        {
            // only add blocks between the old_canonical_root and the new one
            if ancestor_id != old_canonical_root_id {
                let ancestor_block = self.get_block_from_id(ancestor_id).clone();
                if canonical_blocks.is_empty() {
                    // update canonical root
                    self.canonical_root.node_id = ancestor_id.clone();
                    self.canonical_root.state_hash = ancestor_block.state_hash.clone();
                }
                canonical_blocks.push(ancestor_block);
            } else {
                break;
            }
        }

        // sort lowest to highest
        canonical_blocks.reverse();
        Ok(canonical_blocks)
    }

    /// Add new canonical diffs to the ledger
    fn update_ledger(&mut self, canonical_blocks: &Vec<Block>) -> anyhow::Result<()> {
        // apply the new canonical diffs and store each nth resulting ledger
        let mut ledger_diff = LedgerDiff::default();
        for canonical_block in canonical_blocks {
            if let Some(diff) = self.diffs_map.get(&canonical_block.state_hash) {
                ledger_diff.append(diff.clone());
            } else {
                error!(
                    "Block not in diffs map (length {}): {}",
                    canonical_block.blockchain_length, canonical_block.state_hash
                );
            }
        }

        if !ledger_diff.account_diffs.is_empty() {
            self.ledger._apply_diff(&ledger_diff)?;
        }
        Ok(())
    }

    /// Add new canonical ledgers to the ledger store
    fn update_ledger_store(&self, canonical_blocks: &Vec<Block>) -> anyhow::Result<()> {
        if let Some(indexer_store) = self.indexer_store.as_ref() {
            for canonical_block in canonical_blocks {
                if canonical_block.blockchain_length % self.ledger_cadence == 0 {
                    indexer_store.add_staged_ledger_at_state_hash(
                        &canonical_block.state_hash,
                        self.ledger.clone(),
                    )?;
                }
            }
        }
        Ok(())
    }

    /// Remove diffs corresponding to blocks at or beneath the height of the new
    /// canonical root
    fn prune_diffs_map(&mut self, old_canonical_root_id: &NodeId) -> anyhow::Result<()> {
        for node_id in self
            .root_branch
            .branches
            .traverse_level_order_ids(old_canonical_root_id)
            .unwrap()
        {
            let block = self.get_block_from_id(&node_id);
            if block != self.canonical_root_block()
                && block.height <= self.canonical_root_block().height
            {
                self.diffs_map.remove(&block.state_hash.clone());
            }
        }
        Ok(())
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
            best_tip_length: self.best_tip_block().blockchain_length,
            canonical_root_hash: self.canonical_root_block().state_hash.0.clone(),
            canonical_root_length: self.canonical_root_block().blockchain_length,
            root_hash: self.root_branch.root_block().state_hash.0.clone(),
            root_height: self.root_branch.height(),
            root_length: self.root_branch.len(),
            num_leaves: self.root_branch.leaves().len() as u32,
            num_dangling: self.dangling_branches.len() as u32,
            max_dangling_height,
            max_dangling_length,
        };
        let max_staking_ledger_epoch = self.staking_ledgers.keys().max().cloned();
        SummaryShort {
            witness_tree,
            max_staking_ledger_epoch,
            uptime: Instant::now() - self.init_time,
            blocks_processed: self.blocks_processed,
            max_staking_ledger_hash: self
                .staking_ledgers
                .get(&max_staking_ledger_epoch.unwrap_or(0))
                .cloned()
                .map(|h| h.0),
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
            best_tip_length: self.best_tip_block().blockchain_length,
            canonical_root_hash: self.canonical_root_block().state_hash.0.clone(),
            canonical_root_length: self.canonical_root_block().blockchain_length,
            root_hash: self.root_branch.root_block().state_hash.0.clone(),
            root_height: self.root_branch.height(),
            root_length: self.root_branch.len(),
            num_leaves: self.root_branch.leaves().len() as u32,
            num_dangling: self.dangling_branches.len() as u32,
            max_dangling_height,
            max_dangling_length,
            witness_tree: format!("{self}"),
        };
        let max_staking_ledger_epoch = self.staking_ledgers.keys().max().cloned();
        SummaryVerbose {
            witness_tree,
            max_staking_ledger_epoch,
            uptime: Instant::now() - self.init_time,
            blocks_processed: self.blocks_processed,
            max_staking_ledger_hash: self
                .staking_ledgers
                .get(&max_staking_ledger_epoch.unwrap_or(0))
                .cloned()
                .map(|h| h.0),
            db_stats: db_stats_str.map(|s| DbStats::from_str(&format!("{mem}\n{s}")).unwrap()),
        }
    }

    fn should_report_from_block_count(&self, block_parser: &BlockParser) -> bool {
        self.blocks_processed > 0 && self.blocks_processed % self.reporting_freq == 0
            || self.blocks_processed == block_parser.num_deep_canonical_blocks + 1
    }

    fn report_from_block_count(&self, block_parser: &mut BlockParser, total_time: Instant) {
        if self.should_report_from_block_count(block_parser) {
            let elapsed = total_time.elapsed().as_secs();
            let block_rate = self.blocks_processed as f64 / elapsed as f64;
            let bytes_rate = if elapsed != 0 {
                self.bytes_processed / elapsed
            } else {
                u64::MAX
            };
            info!(
                "{}/{} blocks ({:?}/{:?}) parsed and applied in {}",
                self.blocks_processed,
                block_parser.total_num_blocks + 1,
                bytesize::ByteSize::b(self.bytes_processed),
                bytesize::ByteSize::b(block_parser.total_num_bytes + self.genesis_bytes),
                pretty_print_duration(total_time.elapsed()),
            );
            debug!(
                "Rate: {block_rate} blocks/s ({}/s)",
                bytesize::ByteSize::b(bytes_rate)
            );

            let dur = Duration::from_secs(
                block_parser
                    .total_num_bytes
                    .saturating_sub(self.bytes_processed)
                    / bytes_rate,
            );
            if !dur.is_zero() {
                info!("Estimated time remaining: {}", pretty_print_duration(dur));
            }
        }
    }

    fn report_progress(
        &self,
        block_parser: &BlockParser,
        step_time: Instant,
        total_time: Instant,
    ) -> anyhow::Result<()> {
        if self.should_report_from_block_count(block_parser)
            || step_time.elapsed().as_secs() > BLOCK_REPORTING_FREQ_SEC
        {
            let elapsed = total_time.elapsed().as_secs();
            let best_tip: BlockWithoutHeight = self.best_tip_block().clone().into();
            let block_rate = self.blocks_processed as f64 / elapsed as f64;
            let bytes_rate = if elapsed != 0 {
                self.bytes_processed / elapsed
            } else {
                u64::MAX
            };
            info!(
                "Parsed and added {}/{} blocks ({:?}/{:?}) to the witness tree in {}",
                self.blocks_processed,
                block_parser.total_num_blocks + 1,
                bytesize::ByteSize::b(self.bytes_processed),
                bytesize::ByteSize::b(block_parser.total_num_bytes + self.genesis_bytes),
                pretty_print_duration(total_time.elapsed()),
            );
            debug!("Root height:       {}", self.root_branch.height());
            debug!("Root length:       {}", self.root_branch.len());
            debug!(
                "Rate:              {block_rate} blocks/s ({}/s)",
                bytesize::ByteSize::b(bytes_rate)
            );
            info!("Current best tip {}", best_tip.summary());
            let dur = Duration::from_secs(
                block_parser
                    .total_num_bytes
                    .saturating_sub(self.bytes_processed)
                    / bytes_rate,
            );
            if !dur.is_zero() {
                info!("Estimated time remaining: {}", pretty_print_duration(dur));
            }
        }
        Ok(())
    }
}

/// Checks if the block is the parent of the branch's root
fn is_reverse_extension(branch: &Branch, precomputed_block: &PrecomputedBlock) -> bool {
    precomputed_block.state_hash() == branch.root_block().parent_hash
}

impl std::fmt::Display for IndexerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Root branch ===")?;
        writeln!(f, "{}", self.root_branch)?;

        if !self.dangling_branches.is_empty() {
            writeln!(f, "=== Dangling branches ===")?;
            for (n, branch) in self.dangling_branches.iter().enumerate() {
                writeln!(f, "Dangling branch {n}:")?;
                writeln!(f, "{branch}")?;
            }
        }
        Ok(())
    }
}
