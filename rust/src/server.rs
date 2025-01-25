use crate::{
    block::{self, parser::BlockParser, precomputed::PcbVersion, BlockHash},
    chain::{chain_id, ChainId, Network},
    constants::*,
    ledger::{
        genesis::{GenesisConstants, GenesisLedger},
        staking::{self, StakingLedger},
        store::staking::StakingLedgerStore,
    },
    state::{IndexerState, IndexerStateConfig},
    store::IndexerStore,
    unix_socket_server::{create_socket_listener, handle_connection},
};
use log::{debug, error, info, trace, warn};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use speedb::checkpoint::Checkpoint;
use std::{
    collections::HashSet,
    fs,
    os::unix::fs::MetadataExt,
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

#[derive(Clone, Debug)]
pub struct IndexerVersion {
    pub network: Network,
    pub version: PcbVersion,
    pub chain_id: ChainId,
    pub genesis: GenesisVersion,
}

#[derive(Clone, Debug)]
pub struct GenesisVersion {
    pub state_hash: BlockHash,
    pub prev_hash: BlockHash,
    pub blockchain_lenth: u32,
    pub global_slot: u32,
    pub last_vrf_output: String,
}

#[derive(Clone, Debug)]
pub struct IndexerConfiguration {
    pub genesis_ledger: GenesisLedger,
    pub genesis_version: GenesisVersion,
    pub genesis_constants: GenesisConstants,
    pub constraint_system_digests: Vec<String>,
    pub protocol_txn_version_digest: Option<String>,
    pub protocol_network_version_digest: Option<String>,
    pub version: PcbVersion,
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

#[derive(Debug, Clone)]
pub enum InitializationMode {
    BuildDB,
    Replay,
    Sync,
}

/// Initializes indexer database
///
/// The purpose of this mode is to create a known good initial
/// database so that it may be used and shared with other Mina
/// Indexers
pub async fn initialize_indexer_database(
    config: IndexerConfiguration,
    store: &Arc<IndexerStore>,
) -> anyhow::Result<()> {
    let state = initialize(config, store).await.unwrap_or_else(|e| {
        error!("Failed to initialize mina indexer state: {e}");
        std::process::exit(1);
    });
    if let Some(indexer_store) = state.indexer_store.as_ref() {
        indexer_store.database.cancel_all_background_work(true);
    }
    Ok(())
}

/// Initializes witness tree, connects database, starts UDS server & runs the
/// indexer
pub async fn start_indexer(
    subsys: SubsystemHandle,
    config: IndexerConfiguration,
    store: Arc<IndexerStore>,
) -> anyhow::Result<()> {
    let blocks_dir = config.blocks_dir.clone();
    let staking_ledgers_dir = config.staking_ledgers_dir.clone();
    let fetch_new_blocks_delay = config.fetch_new_blocks_delay;
    let fetch_new_blocks_exe = config.fetch_new_blocks_exe.clone();
    let missing_block_recovery_delay = config.missing_block_recovery_delay;
    let missing_block_recovery_exe = config.missing_block_recovery_exe.clone();
    let missing_block_recovery_batch = config.missing_block_recovery_batch;
    let domain_socket_path = config.domain_socket_path.clone();

    // initialize witness tree & connect database
    let state = Arc::new(RwLock::new(
        initialize(config, &store).await.unwrap_or_else(|e| {
            error!("Failed to initialize mina indexer state: {e}");
            std::process::exit(1);
        }),
    ));

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

async fn initialize(
    config: IndexerConfiguration,
    store: &Arc<IndexerStore>,
) -> anyhow::Result<IndexerState> {
    info!("Initializing mina indexer database");
    let db_path = store.db_path.clone();
    let IndexerConfiguration {
        genesis_ledger,
        genesis_version,
        blocks_dir,
        staking_ledgers_dir,
        prune_interval,
        canonical_threshold,
        canonical_update_threshold,
        initialization_mode,
        ledger_cadence,
        reporting_freq,
        genesis_constants,
        constraint_system_digests,
        protocol_txn_version_digest,
        protocol_network_version_digest,
        version,
        do_not_ingest_orphan_blocks,
        ..
    } = config;

    if let Some(ref blocks_dir) = blocks_dir {
        if let Err(e) = fs::create_dir_all(blocks_dir) {
            error!("Failed to create blocks directory in {blocks_dir:#?}: {e}");
            process::exit(1);
        }
    }
    if let Some(ref staking_ledgers_dir) = staking_ledgers_dir {
        if let Err(e) = fs::create_dir_all(staking_ledgers_dir) {
            error!("Failed to create staking ledgers directory in {staking_ledgers_dir:#?}: {e}");
            process::exit(1);
        }
    }

    let chain_id = chain_id(
        &genesis_version.state_hash.0,
        &[
            genesis_constants.k.unwrap(),
            genesis_constants.slots_per_epoch.unwrap(),
            genesis_constants.slots_per_sub_window.unwrap(),
            genesis_constants.delta.unwrap(),
            genesis_constants.txpool_max_size.unwrap(),
        ],
        constraint_system_digests
            .iter()
            .map(|x| x.as_str())
            .collect::<Vec<&str>>()
            .as_slice(),
        MAINNET_GENESIS_TIMESTAMP as i64,
        protocol_txn_version_digest.as_ref().map(|d| d as &str),
        protocol_network_version_digest.as_ref().map(|d| d as &str),
    );
    let indexer_version = IndexerVersion {
        network: Network::Mainnet,
        version: version.clone(),
        chain_id,
        genesis: genesis_version,
    };
    let state_config = IndexerStateConfig {
        indexer_store: store.clone(),
        version: indexer_version.clone(),
        genesis_ledger: genesis_ledger.clone(),
        transition_frontier_length: MAINNET_TRANSITION_FRONTIER_K,
        do_not_ingest_orphan_blocks,
        prune_interval,
        canonical_threshold,
        canonical_update_threshold,
        ledger_cadence,
        reporting_freq,
        using_hardfork_ledger: false,
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
            error!("Failed to ingest staking ledger {staking_ledgers_dir:#?}: {e}");
        }
    }

    // build witness tree & ingest precomputed blocks
    match initialization_mode {
        InitializationMode::BuildDB => {
            if let Some(ref blocks_dir) = blocks_dir {
                let mut block_parser = BlockParser::new_with_canonical_chain_discovery(
                    blocks_dir,
                    version,
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
                    version: indexer_version,
                    genesis_ledger,
                    transition_frontier_length: MAINNET_TRANSITION_FRONTIER_K,
                    prune_interval,
                    canonical_threshold,
                    canonical_update_threshold,
                    ledger_cadence,
                    reporting_freq,
                    do_not_ingest_orphan_blocks,
                    using_hardfork_ledger: false,
                })
            {
                let min_length_filter = state.replay_events(replay_state)?;
                if let Some(ref blocks_dir) = blocks_dir {
                    let mut block_parser = BlockParser::new_length_sorted_min_filtered(
                        blocks_dir,
                        version,
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
                    version,
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

#[cfg(target_os = "linux")]
fn matches_event_kind(kind: EventKind) -> bool {
    matches!(
        kind,
        EventKind::Access(notify::event::AccessKind::Close(
            notify::event::AccessMode::Write
        )) | EventKind::Create(notify::event::CreateKind::File)
            | EventKind::Modify(notify::event::ModifyKind::Name(_))
    )
}

#[cfg(target_os = "macos")]
fn matches_event_kind(kind: EventKind) -> bool {
    matches!(
        kind,
        EventKind::Modify(notify::event::ModifyKind::Data(
            notify::event::DataChange::Content
        )) | EventKind::Modify(notify::event::ModifyKind::Name(_))
            | EventKind::Create(notify::event::CreateKind::File)
    )
}

fn is_hard_link<P: AsRef<Path>>(path: P) -> bool {
    match fs::metadata(path) {
        Ok(metadata) => metadata.nlink() > 1,
        Err(_) => false,
    }
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

    let fetch_new_blocks_delay = fetch_new_blocks_opts.as_ref().map(|f| f.delay);
    let fetch_new_blocks_exe = fetch_new_blocks_opts.as_ref().map(|f| f.exe.clone());
    let missing_block_recovery_delay = missing_block_recovery.as_ref().map(|m| m.delay);
    let missing_block_recovery_exe = missing_block_recovery.as_ref().map(|m| m.exe.clone());
    let missing_block_recovery_batch = missing_block_recovery.map_or(false, |m| m.batch);
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
                        fetch_new_blocks(&state, &blocks_dir, fetch_new_blocks_exe).await
                    }
                }
            }

            // recover missing blocks
            _ = tokio::time::sleep(std::time::Duration::from_secs(missing_block_recovery_delay.unwrap_or(180))) => {
                if let Some(ref blocks_dir) = blocks_dir {
                    if let Some(ref missing_block_recovery_exe) = missing_block_recovery_exe {
                        recover_missing_blocks(&state, &blocks_dir, missing_block_recovery_exe, missing_block_recovery_batch).await
                    }
                }
            }
        }
    }

    let state = state.write().await;
    if let Some(store) = state.indexer_store.as_ref() {
        info!("Canceling db background work");
        store.database.cancel_all_background_work(true)
    }
    info!("Filesystem watchers successfully shutdown");
    Ok(())
}

async fn retry_parse_staking_ledger(
    path: &Path,
    genesis_state_hash: BlockHash,
) -> anyhow::Result<StakingLedger> {
    for attempt in 1..=5 {
        match StakingLedger::parse_file(path, genesis_state_hash.clone()).await {
            Ok(ledger) => return Ok(ledger),
            Err(e) if attempt < 5 => {
                warn!("Attempt {attempt} failed: {e}. Retrying in 1 second...");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Err(e) => panic!("Failed after {attempt} attempts: {e}"),
        }
    }
    panic!("All attempts to parse the staking ledger failed.")
}

/// Precomputed block & staking ledger event handler
async fn process_event(event: Event, state: &Arc<RwLock<IndexerState>>) -> anyhow::Result<()> {
    trace!("Event: {event:?}");
    if matches_event_kind(event.kind) {
        for path in event.paths {
            if !is_hard_link(&path)
                && matches!(
                    event.kind,
                    EventKind::Create(notify::event::CreateKind::File)
                )
            {
                debug!("Ignore create file event when the file isn't a hard link");
                // This potentially can cause an EOF when parsing the
                // files because it many not be fully written
                // yet. Instead use the close event as a signal to
                // process
                continue;
            }
            if block::is_valid_block_file(&path) {
                debug!("Valid precomputed block file: {}", path.display());
                match IndexerState::parse_file(state, &path).await {
                    Ok(block) => {
                        // Acquire write lock
                        let mut state = state.write().await;

                        // check if the block is already in the witness tree
                        if state.diffs_map.contains_key(&block.state_hash()) {
                            return Ok(info!(
                                "Block is already present in the witness tree {}",
                                block.summary()
                            ));
                        }

                        // if the block isn't in the witness tree, pipeline it
                        match state.block_pipeline(&block, path.metadata()?.len()) {
                            Ok(is_added) => {
                                if is_added {
                                    info!("Added block {}", block.summary())
                                }
                            }
                            Err(e) => error!("Error adding block: {e}"),
                        }
                    }
                    Err(e) => error!("Error parsing precomputed block: {e}"),
                }
            } else if staking::is_valid_ledger_file(&path) {
                // acquire state write lock
                let version = state.read().await.version.clone();
                let state = state.write().await;
                if let Some(store) = state.indexer_store.as_ref() {
                    match retry_parse_staking_ledger(&path, version.genesis.state_hash.clone())
                        .await
                    {
                        Ok(staking_ledger) => {
                            let epoch = staking_ledger.epoch;
                            let ledger_hash = staking_ledger.ledger_hash.clone();
                            let ledger_summary = staking_ledger.summary();

                            info!("Adding staking ledger {ledger_summary}");
                            store
                                .add_staking_ledger(
                                    staking_ledger,
                                    &state.version.genesis.state_hash,
                                )
                                .unwrap_or_else(|e| {
                                    error!("Error adding staking ledger {ledger_summary} {e}")
                                });
                            let mut staking_ledgers = state.staking_ledgers.lock().unwrap();
                            staking_ledgers.insert(epoch, ledger_hash);
                        }
                        Err(e) => {
                            error!("Error parsing staking ledger: {e}")
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

/// Fetch new blocks
async fn fetch_new_blocks(
    state: &Arc<RwLock<IndexerState>>,
    blocks_dir: impl AsRef<Path>,
    fetch_new_blocks_exe: impl AsRef<Path>,
) {
    let state = state.read().await;
    let network = state.version.network.clone();
    let new_block_length = state.best_tip_block().blockchain_length + 1;
    let mut c = std::process::Command::new(fetch_new_blocks_exe.as_ref().display().to_string());
    let cmd = c.args([
        &network.to_string(),
        &new_block_length.to_string(),
        &blocks_dir.as_ref().display().to_string(),
    ]);
    match cmd.output() {
        Ok(output) => {
            use std::io::*;
            stdout().write_all(&output.stdout).ok();
            stderr().write_all(&output.stderr).ok();
        }
        Err(e) => error!(
            "Error fetching new blocks: {e}, pgm: {}, args: {:?}",
            cmd.get_program().to_str().unwrap(),
            cmd.get_args()
                .map(|arg| arg.to_str().unwrap())
                .collect::<Vec<&str>>()
        ),
    }
}

/// Recovers missing blocks
async fn recover_missing_blocks(
    state: &Arc<RwLock<IndexerState>>,
    blocks_dir: impl AsRef<Path>,
    missing_block_recovery_exe: impl AsRef<Path>,
    batch_recovery: bool,
) {
    let state = state.read().await;
    let network = state.version.network.clone();
    let missing_parent_lengths: HashSet<u32> = state
        .dangling_branches
        .iter()
        .map(|b| b.root_block().blockchain_length.saturating_sub(1))
        .collect();
    if missing_parent_lengths.is_empty() {
        return;
    }

    let run_missing_blocks_recovery = |blockchain_length: u32| {
        let mut c =
            std::process::Command::new(missing_block_recovery_exe.as_ref().display().to_string());
        let cmd = c.args([
            &network.to_string(),
            &blockchain_length.to_string(),
            &blocks_dir.as_ref().display().to_string(),
        ]);
        match cmd.output() {
            Ok(output) => {
                use std::io::*;
                stdout().write_all(&output.stdout).unwrap();
                stderr().write_all(&output.stderr).unwrap();
            }
            Err(e) => error!(
                "Error recovery missing block: {e}, pgm: {}, args: {:?}",
                cmd.get_program().to_str().unwrap(),
                cmd.get_args()
                    .map(|arg| arg.to_str().unwrap())
                    .collect::<Vec<&str>>()
            ),
        }
    };

    debug!("Getting missing parent blocks of dangling roots");
    let min_missing_length = missing_parent_lengths.iter().min().cloned();
    let max_missing_length = missing_parent_lengths.iter().max().cloned();
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
}

impl GenesisVersion {
    pub fn v1() -> Self {
        Self {
            state_hash: MAINNET_GENESIS_HASH.into(),
            prev_hash: MAINNET_GENESIS_PREV_STATE_HASH.into(),
            last_vrf_output: MAINNET_GENESIS_LAST_VRF_OUTPUT.into(),
            blockchain_lenth: 1,
            global_slot: 0,
        }
    }

    pub fn v2() -> Self {
        Self {
            state_hash: HARDFORK_GENESIS_HASH.into(),
            prev_hash: HARDFORK_GENESIS_PREV_STATE_HASH.into(),
            last_vrf_output: HARDFORK_GENESIS_LAST_VRF_OUTPUT.into(),
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
