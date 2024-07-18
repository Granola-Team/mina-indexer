use crate::{
    block::{
        self,
        parser::BlockParser,
        precomputed::{PcbVersion, PrecomputedBlock},
        BlockHash,
    },
    chain::{chain_id, ChainId, Network},
    constants::*,
    ledger::{
        genesis::{GenesisConstants, GenesisLedger},
        staking::{self, StakingLedger},
        store::LedgerStore,
    },
    state::{IndexerState, IndexerStateConfig},
    store::IndexerStore,
    unix_socket_server::{create_socket_listener, handle_connection},
};
use log::{debug, error, info, trace};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use speedb::checkpoint::Checkpoint;
use std::{
    collections::HashSet,
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process,
    sync::Arc,
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
    pub genesis_state_hash: BlockHash,
}

#[derive(Clone, Debug)]
pub struct IndexerConfiguration {
    pub genesis_ledger: GenesisLedger,
    pub genesis_hash: BlockHash,
    pub genesis_constants: GenesisConstants,
    pub constraint_system_digests: Vec<String>,
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
    pub missing_block_recovery_exe: Option<PathBuf>,
    pub missing_block_recovery_delay: Option<u64>,
    pub missing_block_recovery_batch: bool,
}

#[derive(Debug, Clone)]
pub enum InitializationMode {
    New,
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
    run_indexer(
        &subsys,
        blocks_dir,
        staking_ledgers_dir,
        missing_block_recovery_delay,
        missing_block_recovery_exe,
        missing_block_recovery_batch,
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
    info!("Initializing new mina indexer");
    let db_path = store.db_path.clone();
    let IndexerConfiguration {
        genesis_ledger,
        genesis_hash,
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
        version,
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
        &genesis_hash.0,
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
    );
    let indexer_version = IndexerVersion::new(&Network::Mainnet, &chain_id, &genesis_hash);
    let state_config = IndexerStateConfig {
        genesis_hash: genesis_hash.clone(),
        indexer_store: store.clone(),
        version: indexer_version.clone(),
        genesis_ledger: genesis_ledger.clone(),
        transition_frontier_length: MAINNET_TRANSITION_FRONTIER_K,
        prune_interval,
        canonical_threshold,
        canonical_update_threshold,
        ledger_cadence,
        reporting_freq,
    };

    let mut state = match initialization_mode {
        InitializationMode::New => {
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
        InitializationMode::New => {
            if let Some(ref blocks_dir) = blocks_dir {
                let mut block_parser = BlockParser::new_with_canonical_chain_discovery(
                    blocks_dir,
                    version,
                    canonical_threshold,
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
                    genesis_hash: genesis_hash.clone(),
                    indexer_store: store.clone(),
                    version: indexer_version.clone(),
                    genesis_ledger: genesis_ledger.clone(),
                    transition_frontier_length: MAINNET_TRANSITION_FRONTIER_K,
                    prune_interval,
                    canonical_threshold,
                    canonical_update_threshold,
                    ledger_cadence,
                    reporting_freq,
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
    )
}

fn is_hard_link<P: AsRef<Path>>(path: P) -> bool {
    match fs::metadata(path) {
        Ok(metadata) => metadata.nlink() > 1,
        Err(_) => false,
    }
}

/// Starts filesystem watchers & runs the mina indexer
async fn run_indexer<P: AsRef<Path>>(
    subsys: &SubsystemHandle,
    blocks_dir: Option<P>,
    staking_ledgers_dir: Option<P>,
    missing_block_recovery_delay: Option<u64>,
    missing_block_recovery_exe: Option<PathBuf>,
    missing_block_recovery_batch: bool,
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
                let mut version = state.read().await.version.clone();
                match PrecomputedBlock::parse_file(&path, version.version.clone()) {
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

                        // check for block parser version update
                        if state.version.genesis_state_hash.0 != *MAINNET_GENESIS_HASH {
                            trace!("Changing block parser from {}", version.version);
                            version.version.update()?;

                            trace!("Block parser changed to {}", version.version);
                            version.genesis_state_hash = state.version.genesis_state_hash.clone();
                        }
                    }
                    Err(e) => error!("Error parsing precomputed block: {e}"),
                }
            } else if staking::is_valid_ledger_file(&path) {
                // acquire state write lock
                let version = state.read().await.version.clone();
                let mut state = state.write().await;
                if let Some(store) = state.indexer_store.as_ref() {
                    match StakingLedger::parse_file(&path, version.genesis_state_hash.clone()).await
                    {
                        Ok(staking_ledger) => {
                            let epoch = staking_ledger.epoch;
                            let ledger_hash = staking_ledger.ledger_hash.clone();
                            let ledger_summary = staking_ledger.summary();

                            info!("Adding staking ledger {ledger_summary}");
                            store
                                .add_staking_ledger(
                                    staking_ledger,
                                    &state.version.genesis_state_hash,
                                )
                                .unwrap_or_else(|e| {
                                    error!("Error adding staking ledger {ledger_summary} {e}")
                                });
                            state.staking_ledgers.insert(epoch, ledger_hash);
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

/// Recovers missing blocks
async fn recover_missing_blocks(
    state: &Arc<RwLock<IndexerState>>,
    block_watch_dir: impl AsRef<Path>,
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
            &block_watch_dir.as_ref().display().to_string(),
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

impl IndexerVersion {
    pub fn new(network: &Network, chain_id: &ChainId, genesis_state_hash: &BlockHash) -> Self {
        Self {
            version: PcbVersion::default(),
            network: network.clone(),
            chain_id: chain_id.clone(),
            genesis_state_hash: genesis_state_hash.clone(),
        }
    }

    pub fn new_testing() -> Self {
        let chain_id = chain_id(
            MAINNET_GENESIS_HASH,
            MAINNET_PROTOCOL_CONSTANTS,
            MAINNET_CONSTRAINT_SYSTEM_DIGESTS,
        );
        Self::new(&Network::Mainnet, &chain_id, &MAINNET_GENESIS_HASH.into())
    }
}

fn log_dirs_msg(blocks_dir: Option<&PathBuf>, staking_ledgers_dir: Option<&PathBuf>) {
    match (blocks_dir, staking_ledgers_dir) {
        (Some(blocks_dir), Some(staking_ledgers_dir)) => info!(
            "Initializing indexer from blocks in {blocks_dir:#?} and staking ledgers in {staking_ledgers_dir:#?}"
        ),
        (Some(blocks_dir), None) => info!(
            "Initializing indexer from blocks in {blocks_dir:#?}"
        ),
        (None, Some(staking_ledgers_dir)) => info!(
            "Initializing indexer from staking ledgers in {staking_ledgers_dir:#?}"
        ),
        (None, None) => info!("Initializing indexer without blocks and staking ledgers"),
    }
}
