//! Server & file

use crate::{
    base::state_hash::StateHash,
    block::{
        self,
        parser::BlockParser,
        precomputed::{PcbVersion, PrecomputedBlock},
        vrf_output::VrfOutput,
    },
    chain::{ChainId, Network},
    cli::server::ServerArgsJson,
    constants::*,
    ledger::{
        genesis::GenesisLedger, staking::StakingLedger, store::staking::StakingLedgerStore,
        LedgerHash,
    },
    state::{IndexerState, IndexerStateConfig},
    store::IndexerStore,
    unix_socket_server::{create_socket_listener, handle_connection},
    utility::functions::extract_network_height_hash,
};
use log::{debug, error, info, trace, warn};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use speedb::checkpoint::Checkpoint;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process,
    sync::Arc,
    time::Duration,
};
use tokio::{
    runtime::Handle,
    sync::{mpsc, RwLock},
};
use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IndexerVersion {
    pub network: Network,
    pub version: PcbVersion,
    pub chain_id: ChainId,
    pub genesis: GenesisVersion,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GenesisVersion {
    pub state_hash: StateHash,
    pub prev_hash: StateHash,
    pub blockchain_lenth: u32,
    pub global_slot: u32,
    pub last_vrf_output: VrfOutput,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IndexerConfiguration {
    pub genesis_ledger: GenesisLedger,
    pub version: IndexerVersion,
    pub blocks_dir: Option<PathBuf>,
    pub staking_ledgers_dir: Option<PathBuf>,
    pub prune_interval: u32,
    pub canonical_threshold: u32,
    pub canonical_update_threshold: u32,
    pub initialization_mode: InitializationMode,
    pub ledger_cadence: u32,
    pub reporting_freq: u32,
    pub domain_socket_path: PathBuf,
    pub do_not_ingest_orphan_blocks: bool,
    pub fetch_new_blocks_exe: Option<PathBuf>,
    pub fetch_new_blocks_delay: Option<u64>,
    pub missing_block_recovery_exe: Option<PathBuf>,
    pub missing_block_recovery_delay: Option<u64>,
    pub missing_block_recovery_batch: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum InitializationMode {
    BuildDB,
    Replay,
    Sync,
}

///////////
// impls //
///////////

impl IndexerConfiguration {
    /// Initializes indexer database
    ///
    /// The purpose of this mode is to create a known good initial
    /// database so that it may be used and shared with other Mina
    /// Indexers
    pub async fn initialize_indexer_database(
        self,
        store: &Arc<IndexerStore>,
    ) -> anyhow::Result<()> {
        let state = self.initialize(store).await.unwrap_or_else(|e| {
            error!("Failed to initialize mina indexer store: {e}");
            std::process::exit(1);
        });

        if let Some(indexer_store) = state.indexer_store.as_ref() {
            indexer_store.database.cancel_all_background_work(true);
        }

        Ok(())
    }

    /// Initializes the indexer with the given config & store
    async fn initialize(self, store: &Arc<IndexerStore>) -> anyhow::Result<IndexerState> {
        debug!("Initializing mina indexer database");
        let db_path = store.db_path.clone();

        // read the config from the store if it exists or write it
        let IndexerConfiguration {
            genesis_ledger,
            blocks_dir,
            staking_ledgers_dir,
            prune_interval,
            canonical_threshold,
            canonical_update_threshold,
            initialization_mode,
            ledger_cadence,
            reporting_freq,
            version,
            do_not_ingest_orphan_blocks,
            ..
        } = self;

        // blocks dir
        if let Some(ref blocks_dir) = blocks_dir {
            if let Err(e) = fs::create_dir_all(blocks_dir) {
                error!(
                    "Failed to create blocks directory in {:#?}: {}",
                    blocks_dir, e
                );
                process::exit(1);
            }
        }

        // staking ledger dir
        if let Some(ref staking_ledgers_dir) = staking_ledgers_dir {
            if let Err(e) = fs::create_dir_all(staking_ledgers_dir) {
                error!(
                    "Failed to create staking ledgers directory in {:#?}: {}",
                    staking_ledgers_dir, e
                );
                process::exit(1);
            }
        }

        let pcb_version = version.version.to_owned();
        let state_config = IndexerStateConfig {
            indexer_store: store.clone(),
            version: version.clone(),
            genesis_ledger: genesis_ledger.clone(),
            transition_frontier_length: MAINNET_TRANSITION_FRONTIER_K,
            do_not_ingest_orphan_blocks,
            prune_interval,
            canonical_threshold,
            canonical_update_threshold,
            ledger_cadence,
            reporting_freq,
        };

        let mut state = match initialization_mode {
            InitializationMode::BuildDB => {
                log_dirs_msg(blocks_dir.as_ref(), staking_ledgers_dir.as_ref());
                IndexerState::new_from_config(state_config)?
            }
            InitializationMode::Replay => {
                info!("Replaying indexer events from db at {db_path:#?}");
                IndexerState::new_without_genesis_events(state_config)?
            }
            InitializationMode::Sync => {
                info!("Syncing indexer state from db at {db_path:#?}");
                IndexerState::new_without_genesis_events(state_config)?
            }
        };

        // ingest staking ledgers
        if let Some(ref staking_ledgers_dir) = staking_ledgers_dir {
            if let Err(e) = state
                .add_startup_staking_ledgers_to_store(staking_ledgers_dir)
                .await
            {
                panic!("Failed to ingest staking ledger {staking_ledgers_dir:#?}: {e}");
            }
        }

        // build witness tree & ingest precomputed blocks
        match initialization_mode {
            InitializationMode::BuildDB => {
                if let Some(ref blocks_dir) = blocks_dir {
                    let mut block_parser = BlockParser::new_with_canonical_chain_discovery(
                        blocks_dir,
                        pcb_version,
                        canonical_threshold,
                        do_not_ingest_orphan_blocks,
                        reporting_freq,
                    )
                    .await
                    .unwrap_or_else(|e| panic!("Obtaining block parser failed: {e}"));
                    state
                        .initialize_with_canonical_chain_discovery(&mut block_parser)
                        .await?;
                }
            }
            InitializationMode::Replay => {
                if let Ok(ref replay_state) =
                    IndexerState::new_without_genesis_events(IndexerStateConfig {
                        indexer_store: store.clone(),
                        version,
                        genesis_ledger,
                        transition_frontier_length: MAINNET_TRANSITION_FRONTIER_K,
                        prune_interval,
                        canonical_threshold,
                        canonical_update_threshold,
                        ledger_cadence,
                        reporting_freq,
                        do_not_ingest_orphan_blocks,
                    })
                {
                    let min_length_filter = state.replay_events(replay_state)?;
                    if let Some(ref blocks_dir) = blocks_dir {
                        let mut block_parser = BlockParser::new_length_sorted_min_filtered(
                            blocks_dir,
                            pcb_version,
                            min_length_filter,
                        )?;

                        if block_parser.total_num_blocks > 0 {
                            info!("Adding new blocks from {blocks_dir:#?}");
                            state.add_blocks(&mut block_parser).await?;
                        }
                    }
                }
            }
            InitializationMode::Sync => {
                let min_length_filter = state.sync_from_db()?;
                if let Some(ref blocks_dir) = blocks_dir {
                    let mut block_parser = BlockParser::new_length_sorted_min_filtered(
                        blocks_dir,
                        pcb_version,
                        min_length_filter,
                    )?;

                    if block_parser.total_num_blocks > 0 {
                        info!("Adding new blocks from {blocks_dir:#?}");
                        state.add_blocks(&mut block_parser).await?;
                    }
                }
            }
        }

        // flush/compress database
        let store = state.indexer_store.as_ref().unwrap();
        let temp_checkpoint_dir = store.db_path.join("tmp-checkpoint");

        Checkpoint::new(&store.database)?.create_checkpoint(&temp_checkpoint_dir)?;
        fs::remove_dir_all(&temp_checkpoint_dir)?;

        Ok(state)
    }

    /// Initializes witness tree, connects database, starts UDS server & runs
    /// the indexer
    pub async fn start_indexer(
        self,
        subsys: SubsystemHandle,
        store: Arc<IndexerStore>,
    ) -> anyhow::Result<()> {
        let blocks_dir = self.blocks_dir.clone();
        let staking_ledgers_dir = self.staking_ledgers_dir.clone();
        let fetch_new_blocks_delay = self.fetch_new_blocks_delay;
        let fetch_new_blocks_exe = self.fetch_new_blocks_exe.clone();
        let missing_block_recovery_delay = self.missing_block_recovery_delay;
        let missing_block_recovery_exe = self.missing_block_recovery_exe.clone();
        let missing_block_recovery_batch = self.missing_block_recovery_batch;
        let domain_socket_path = self.domain_socket_path.clone();

        // initialize witness tree & connect database
        let state = Arc::new(RwLock::new(self.initialize(&store).await.unwrap_or_else(
            |e| {
                error!("Failed to initialize mina indexer state: {e}");
                std::process::exit(1);
            },
        )));

        // read-only state
        start_uds_server(&subsys, state.clone(), &domain_socket_path).await?;

        // modifies the state
        let missing_block_recovery =
            missing_block_recovery_exe.map(|exe| MissingBlockRecoveryOptions {
                exe,
                batch: missing_block_recovery_batch,
                delay: missing_block_recovery_delay.unwrap_or(180),
            });
        let fetch_new_blocks = fetch_new_blocks_exe.map(|exe| FetchNewBlocksOptions {
            exe,
            delay: fetch_new_blocks_delay.unwrap_or(180),
        });

        run_indexer(
            &subsys,
            blocks_dir,
            staking_ledgers_dir,
            missing_block_recovery,
            fetch_new_blocks,
            state.clone(),
        )
        .await?;

        Ok(())
    }
}

/// Starts UDS server with read-only state for summary
async fn start_uds_server(
    subsys: &SubsystemHandle,
    state: Arc<RwLock<IndexerState>>,
    domain_socket_path: &Path,
) -> anyhow::Result<()> {
    let listener = create_socket_listener(domain_socket_path);

    subsys.start(SubsystemBuilder::new("Socket Listener", {
        move |subsys| handle_connection(listener, state, subsys)
    }));

    Ok(())
}

fn matches_event_kind(kind: EventKind) -> bool {
    use notify::event::{AccessKind, AccessMode, CreateKind, ModifyKind};

    matches!(
        kind,
        EventKind::Create(CreateKind::File)
            | EventKind::Modify(ModifyKind::Name(_))
            | EventKind::Access(AccessKind::Close(AccessMode::Write))
    )
}

struct MissingBlockRecoveryOptions {
    pub delay: u64,
    pub exe: PathBuf,
    pub batch: bool,
}

struct FetchNewBlocksOptions {
    pub delay: u64,
    pub exe: PathBuf,
}

/// Starts filesystem watchers & runs the mina indexer
async fn run_indexer<P: AsRef<Path>>(
    subsys: &SubsystemHandle,
    blocks_dir: Option<P>,
    staking_ledgers_dir: Option<P>,
    missing_block_recovery: Option<MissingBlockRecoveryOptions>,
    fetch_new_blocks_opts: Option<FetchNewBlocksOptions>,
    state: Arc<RwLock<IndexerState>>,
) -> anyhow::Result<()> {
    // setup fs-based precomputed block & staking ledger watchers
    let (tx, mut rx) = mpsc::channel(4096);
    let rt = Handle::current();
    let mut watcher = RecommendedWatcher::new(
        move |result| {
            let tx = tx.clone();
            rt.spawn(async move {
                if let Err(e) = tx.send(result).await {
                    error!("Failed to send watcher event, closing: {e}");
                    drop(tx);
                }
            });
        },
        Config::default(),
    )?;

    if let Some(ref blocks_dir) = blocks_dir {
        watcher.watch(blocks_dir.as_ref(), RecursiveMode::NonRecursive)?;
        info!(
            "Watching for precomputed blocks in directory: {:#?}",
            blocks_dir.as_ref()
        );
    }

    if let Some(ref staking_ledgers_dir) = staking_ledgers_dir {
        watcher.watch(staking_ledgers_dir.as_ref(), RecursiveMode::NonRecursive)?;
        info!(
            "Watching for staking ledgers in directory: {:#?}",
            staking_ledgers_dir.as_ref()
        );
    }

    // fetch new block options
    let fetch_new_blocks_delay = fetch_new_blocks_opts.as_ref().map(|f| f.delay);
    let fetch_new_blocks_exe = fetch_new_blocks_opts.map(|f| f.exe);

    // missing block recovery options
    let missing_block_recovery_batch = missing_block_recovery.as_ref().is_some_and(|m| m.batch);
    let missing_block_recovery_delay = missing_block_recovery.as_ref().map(|m| m.delay);
    let missing_block_recovery_exe = missing_block_recovery.map(|m| m.exe);

    loop {
        tokio::select! {
            // watch for shutdown signals
            _ = subsys.on_shutdown_requested() => {
                break;
            }

            // watch for precomputed blocks & staking ledgers
            Some(res) = rx.recv() => {
                match res {
                    Ok(event) => process_event(event, &state).await?,
                    Err(e) => {
                        error!("Filesystem watcher error: {e}");
                        break;
                    }
                }
            }

            // fetch new blocks
            _ = tokio::time::sleep(std::time::Duration::from_secs(fetch_new_blocks_delay.unwrap_or(180))) => {
                if let Some(ref blocks_dir) = blocks_dir {
                    if let Some(ref fetch_new_blocks_exe) = fetch_new_blocks_exe {
                        fetch_new_blocks(&state, &blocks_dir, fetch_new_blocks_exe).await?
                    }
                }
            }

            // recover missing blocks
            _ = tokio::time::sleep(std::time::Duration::from_secs(missing_block_recovery_delay.unwrap_or(180))) => {
                if let Some(ref blocks_dir) = blocks_dir {
                    if let Some(ref missing_block_recovery_exe) = missing_block_recovery_exe {
                        recover_missing_blocks(&state, &blocks_dir, missing_block_recovery_exe, missing_block_recovery_batch).await?
                    }
                }
            }
        }
    }

    // shutdown
    let state = state.write().await;
    if let Some(store) = state.indexer_store.as_ref() {
        debug!("Canceling db background work");
        store.database.cancel_all_background_work(true)
    }

    debug!("Filesystem watchers successfully shutdown");
    Ok(())
}

async fn retry_parse_precomputed_block(path: &Path) -> anyhow::Result<PrecomputedBlock> {
    let num_attempts = 5;
    for attempt in 1..num_attempts {
        match PrecomputedBlock::from_path(path) {
            Ok(block) => return Ok(block),
            Err(e) => {
                warn!("Attempt {attempt}: {e}. Retrying in 100ms...");
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }

    panic!(
        "All {} attempts to parse the staking ledger failed.",
        num_attempts
    )
}

async fn retry_parse_staking_ledger(path: &Path) -> anyhow::Result<StakingLedger> {
    let num_attempts = 5;
    for attempt in 1..num_attempts {
        match StakingLedger::parse_file(path).await {
            Ok(ledger) => return Ok(ledger),
            Err(e) => {
                warn!("Attempt {attempt}: {e}. Retrying in 1s...");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    panic!(
        "All {} attempts to parse the staking ledger failed.",
        num_attempts
    )
}

/// Precomputed block & staking ledger event handler
async fn process_event(event: Event, state: &Arc<RwLock<IndexerState>>) -> anyhow::Result<()> {
    trace!("{:?}", event);

    if matches_event_kind(event.kind) {
        for path in event.paths {
            if block::is_valid_block_file(&path) {
                debug!("Valid precomputed block file: {:#?}", path);

                // exit early if present
                if check_block(state, &path).await {
                    return Ok(());
                }

                // if the block isn't in the witness tree, parse & pipeline it
                match retry_parse_precomputed_block(&path).await {
                    Ok(block) => {
                        let mut state = state.write().await;
                        let block_summary = block.summary();

                        match state.block_pipeline(&block, path.metadata()?.len()) {
                            Ok(is_added) => {
                                if is_added {
                                    info!("Added block {}", block_summary)
                                }
                            }
                            Err(e) => error!("Error adding block {}: {}", block_summary, e),
                        }
                    }
                    Err(e) => error!("Error parsing precomputed block: {}", e),
                }
            } else if StakingLedger::is_valid(&path) {
                debug!("Valid staking ledger file: {:#?}", path);

                // exit early if present
                if check_staking_ledger(state, &path).await {
                    return Ok(());
                }

                // if staking ledger is not in the witness tree, parse & add it
                let mut state = state.write().await;
                if let Some(store) = state.indexer_store.as_ref() {
                    match retry_parse_staking_ledger(&path).await {
                        Ok(staking_ledger) => {
                            let epoch = staking_ledger.epoch;
                            let ledger_hash = staking_ledger.ledger_hash.clone();
                            let ledger_summary = staking_ledger.summary();

                            info!("Adding staking ledger {}", ledger_summary);
                            store
                                .add_staking_ledger(
                                    staking_ledger,
                                    &state.version.genesis.state_hash,
                                )
                                .unwrap_or_else(|e| {
                                    error!("Error adding staking ledger {}: {}", ledger_summary, e)
                                });

                            state.staking_ledgers.insert((epoch, ledger_hash));
                        }
                        Err(e) => {
                            error!("Error parsing staking ledger: {}", e)
                        }
                    }
                } else {
                    error!("Indexer store unavailable");
                }
            }
        }
    }

    Ok(())
}

/// Checks if the PCB is already present in the witness tree
async fn check_block(state: &Arc<RwLock<IndexerState>>, path: &Path) -> bool {
    let (network, height, state_hash) = extract_network_height_hash(path);
    let state_hash: StateHash = state_hash.into();
    let ro_state = state.read().await;

    // check if the block is already in the witness tree
    if ro_state.diffs_map.contains_key(&state_hash) {
        info!(
            "Block is already present in the witness tree {}-{}-{}",
            network, height, state_hash
        );

        return true;
    }

    false
}

/// Checks if the staking ledger is already present in the witness tree
async fn check_staking_ledger(state: &Arc<RwLock<IndexerState>>, path: &Path) -> bool {
    let (network, epoch, ledger_hash) = extract_network_height_hash(path);
    let ledger_hash: LedgerHash = ledger_hash.into();
    let ro_state = state.read().await;

    // check if the staking ledger is already in the witness tree
    if ro_state
        .staking_ledgers
        .contains(&(epoch, ledger_hash.clone()))
    {
        info!(
            "Staking ledger is already present in the witness tree {}-{}-{}",
            network, epoch, ledger_hash
        );

        return true;
    }

    false
}

/// Fetch new blocks
async fn fetch_new_blocks(
    state: &Arc<RwLock<IndexerState>>,
    blocks_dir: impl AsRef<Path>,
    fetch_new_blocks_exe: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let state = state.read().await;
    let network = state.version.network.clone();
    let new_block_length = state.best_tip_block().blockchain_length + 1;

    let mut cmd = std::process::Command::new(fetch_new_blocks_exe.as_ref().display().to_string());
    let cmd = cmd.args([
        &network.to_string(),
        &new_block_length.to_string(),
        &blocks_dir.as_ref().display().to_string(),
    ]);

    match cmd.output() {
        Ok(output) => {
            let stdout = String::from_utf8(output.stdout)?;
            let stdout = stdout.trim_end();

            if !stdout.is_empty() {
                info!("{}", stdout);
            }

            let stderr = String::from_utf8(output.stderr)?;
            let stderr = stderr.trim_end();

            if !stderr.is_empty() {
                info!("{}", stderr);
            }
        }
        Err(e) => error!(
            "Error fetching new blocks: {}, pgm: {}, args: {:?}",
            e,
            cmd.get_program().to_str().unwrap(),
            cmd.get_args()
                .map(|arg| arg.to_str().unwrap())
                .collect::<Vec<_>>()
        ),
    }

    Ok(())
}

/// Recovers missing blocks
async fn recover_missing_blocks(
    state: &Arc<RwLock<IndexerState>>,
    blocks_dir: impl AsRef<Path>,
    missing_block_recovery_exe: impl AsRef<Path>,
    batch_recovery: bool,
) -> anyhow::Result<()> {
    let state = state.read().await;
    let network = state.version.network.clone();
    let missing_parent_lengths: HashSet<u32> = state
        .dangling_branches
        .iter()
        .map(|b| b.root_block().blockchain_length.saturating_sub(1))
        .collect();

    // exit early if no missing blocks
    if missing_parent_lengths.is_empty() {
        return Ok(());
    }

    let run_missing_blocks_recovery = |blockchain_length: u32| {
        let mut cmd =
            std::process::Command::new(missing_block_recovery_exe.as_ref().display().to_string());
        let cmd = cmd.args([
            &network.to_string(),
            &blockchain_length.to_string(),
            &blocks_dir.as_ref().display().to_string(),
        ]);

        match cmd.output() {
            Ok(output) => {
                let stdout = String::from_utf8(output.stdout).expect("stdout");
                let stdout = stdout.trim_end();

                if !stdout.is_empty() {
                    info!("{}", stdout);
                }

                let stderr = String::from_utf8(output.stderr).expect("stderr");
                let stderr = stderr.trim_end();

                if !stderr.is_empty() {
                    info!("{}", stderr);
                }
            }
            Err(e) => error!(
                "Error recovery missing block: {}, pgm: {}, args: {:?}",
                e,
                cmd.get_program().to_str().unwrap(),
                cmd.get_args()
                    .map(|arg| arg.to_str().unwrap())
                    .collect::<Vec<_>>()
            ),
        }
    };

    debug!("Getting missing parent blocks of dangling roots");
    let min_missing_length = missing_parent_lengths.iter().min().cloned();
    let max_missing_length = missing_parent_lengths.iter().max().cloned();

    // fetch each missing block
    missing_parent_lengths
        .into_iter()
        .for_each(run_missing_blocks_recovery);

    if batch_recovery {
        let best_tip_length = state.best_tip_block().blockchain_length;
        if let (Some(min), Some(max)) = (min_missing_length, max_missing_length) {
            let min_length = best_tip_length.min(min);
            let max_length = best_tip_length.max(max);
            (min_length..max_length).for_each(run_missing_blocks_recovery)
        }
    }

    Ok(())
}

impl GenesisVersion {
    pub fn v1() -> Self {
        use std::str::FromStr;
        let last_vrf_output =
            VrfOutput::from_str(MAINNET_GENESIS_LAST_VRF_OUTPUT).expect("v1 last vrf output");

        Self {
            state_hash: MAINNET_GENESIS_HASH.into(),
            prev_hash: MAINNET_GENESIS_PREV_STATE_HASH.into(),
            last_vrf_output,
            blockchain_lenth: 1,
            global_slot: 0,
        }
    }

    pub fn v2() -> Self {
        use std::str::FromStr;
        let last_vrf_output =
            VrfOutput::from_str(HARDFORK_GENESIS_LAST_VRF_OUTPUT).expect("v2 last vrf output");

        Self {
            last_vrf_output,
            state_hash: HARDFORK_GENESIS_HASH.into(),
            prev_hash: HARDFORK_GENESIS_PREV_STATE_HASH.into(),
            blockchain_lenth: HARDFORK_GENESIS_BLOCKCHAIN_LENGTH,
            global_slot: HARDFORK_GENESIS_GLOBAL_SLOT,
        }
    }
}

impl IndexerVersion {
    pub fn v1() -> Self {
        Self {
            network: Network::Mainnet,
            version: PcbVersion::V1,
            chain_id: ChainId::v1(),
            genesis: GenesisVersion::v1(),
        }
    }

    pub fn v2() -> Self {
        Self {
            network: Network::Mainnet,
            version: PcbVersion::V2,
            chain_id: ChainId::v2(),
            genesis: GenesisVersion::v2(),
        }
    }
}

impl From<(ServerArgsJson, PathBuf)> for IndexerConfiguration {
    fn from(value: (ServerArgsJson, PathBuf)) -> Self {
        let genesis_ledger = if value.0.genesis_hash == HARDFORK_GENESIS_HASH {
            GenesisLedger::new_v2().expect("v2 genesis ledger")
        } else {
            GenesisLedger::new_v1().expect("v1 genesis ledger")
        };
        let version = if value.0.genesis_hash == HARDFORK_GENESIS_HASH {
            IndexerVersion::v2()
        } else {
            IndexerVersion::v1()
        };

        Self {
            version,
            genesis_ledger,
            domain_socket_path: value.1,
            blocks_dir: value.0.blocks_dir.map(Into::into),
            staking_ledgers_dir: value.0.staking_ledgers_dir.map(Into::into),
            prune_interval: value.0.prune_interval,
            canonical_threshold: value.0.canonical_threshold,
            canonical_update_threshold: value.0.canonical_update_threshold,
            initialization_mode: InitializationMode::Sync,
            ledger_cadence: value.0.ledger_cadence,
            reporting_freq: value.0.reporting_freq,
            do_not_ingest_orphan_blocks: value.0.do_not_ingest_orphan_blocks,
            fetch_new_blocks_exe: value.0.fetch_new_blocks_exe.map(Into::into),
            fetch_new_blocks_delay: value.0.fetch_new_blocks_delay,
            missing_block_recovery_exe: value.0.missing_block_recovery_exe.map(Into::into),
            missing_block_recovery_delay: value.0.missing_block_recovery_delay,
            missing_block_recovery_batch: value.0.missing_block_recovery_batch.unwrap_or_default(),
        }
    }
}

impl Default for IndexerVersion {
    fn default() -> Self {
        Self::v1()
    }
}

fn log_dirs_msg(blocks_dir: Option<&PathBuf>, staking_ledgers_dir: Option<&PathBuf>) {
    match (blocks_dir, staking_ledgers_dir) {
        (Some(blocks_dir), Some(staking_ledgers_dir)) => info!(
            "Initializing database from blocks in {blocks_dir:#?} and staking ledgers in {staking_ledgers_dir:#?}"
        ),
        (Some(blocks_dir), None) => info!(
            "Initializing database from blocks in {blocks_dir:#?}"
        ),
        (None, Some(staking_ledgers_dir)) => info!(
            "Initializing database from staking ledgers in {staking_ledgers_dir:#?}"
        ),
        (None, None) => info!("Initializing database without blocks and staking ledgers"),
    }
}
