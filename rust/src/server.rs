use crate::{
    block::{
        self,
        parser::BlockParser,
        precomputed::{PcbVersion, PrecomputedBlock},
        BlockHash,
    },
    chain_id::{chain_id, ChainId, Network},
    constants::*,
    ledger::{
        genesis::{GenesisConstants, GenesisLedger},
        staking::{self, StakingLedger},
        store::LedgerStore,
    },
    state::{IndexerState, IndexerStateConfig},
    store::IndexerStore,
    unix_socket_server::{self, UnixSocketServer},
};
use log::{debug, error, info, trace};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    runtime::Handle,
    sync::{mpsc, RwLock},
    task::JoinHandle,
};

#[derive(Clone, Debug)]
pub struct IndexerVersion {
    pub network: Network,
    pub chain_id: ChainId,
    pub version: PcbVersion,
    pub history: HashMap<ChainId, PcbVersion>,
}

#[derive(Clone, Debug)]
pub struct IndexerConfiguration {
    pub genesis_ledger: GenesisLedger,
    pub genesis_hash: BlockHash,
    pub genesis_constants: GenesisConstants,
    pub constraint_system_digests: Vec<String>,
    pub version: PcbVersion,
    pub blocks_dir: Option<PathBuf>,
    pub block_watch_dir: PathBuf,
    pub staking_ledgers_dir: Option<PathBuf>,
    pub staking_ledger_watch_dir: PathBuf,
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

pub struct MinaIndexer(JoinHandle<()>);

#[derive(Debug, Clone)]
pub enum InitializationMode {
    New,
    Replay,
    Sync,
}

impl MinaIndexer {
    pub async fn new(
        config: IndexerConfiguration,
        store: Arc<IndexerStore>,
    ) -> anyhow::Result<Self> {
        let block_watch_dir = config.block_watch_dir.clone();
        let staking_ledger_watch_dir = config.staking_ledger_watch_dir.clone();
        let missing_block_recovery_delay = config.missing_block_recovery_delay;
        let missing_block_recovery_exe = config.missing_block_recovery_exe.clone();
        let missing_block_recovery_batch = config.missing_block_recovery_batch;
        let domain_socket_path = config.domain_socket_path.clone();
        Ok(Self(tokio::spawn(async move {
            let state = initialize(config, store).unwrap_or_else(|e| {
                error!("Error in server initialization: {}", e);
                std::process::exit(1);
            });
            let state = Arc::new(RwLock::new(state));

            // Needs read-only state for summary
            let uds_state = state.clone();
            tokio::spawn(async move {
                unix_socket_server::run(
                    UnixSocketServer::new(uds_state, domain_socket_path),
                    wait_for_signal(),
                )
                .await;
            });

            // Modifies the state
            if let Err(e) = run(
                block_watch_dir,
                staking_ledger_watch_dir,
                missing_block_recovery_delay,
                missing_block_recovery_exe,
                missing_block_recovery_batch,
                state,
            )
            .await
            {
                error!("Error in server run: {}", e);
                std::process::exit(1);
            }
        })))
    }

    pub async fn await_loop(self) {
        let _ = self.0.await;
    }
}

async fn wait_for_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut term = signal(SignalKind::terminate()).expect("sigterm signal handler registered");
    let mut int = signal(SignalKind::interrupt()).expect("sigitm signal handler registered");
    tokio::select! {
        _ = term.recv() => {
            trace!("Received SIGTERM");
        },
        _ = int.recv() => {
            info!("Received SIGINT");
        },
    }
}

pub fn initialize(
    config: IndexerConfiguration,
    store: Arc<IndexerStore>,
) -> anyhow::Result<IndexerState> {
    info!("Starting mina-indexer server");

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

    blocks_dir
        .iter()
        .for_each(|d| fs::create_dir_all(d.clone()).expect("blocks dir"));
    staking_ledgers_dir
        .iter()
        .for_each(|d| fs::create_dir_all(d.clone()).expect("ledgers dir"));

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
    let state_config = IndexerStateConfig {
        genesis_hash,
        indexer_store: store,
        genesis_ledger: genesis_ledger.clone(),
        version: IndexerVersion::new(&Network::Mainnet, &chain_id),
        transition_frontier_length: MAINNET_TRANSITION_FRONTIER_K,
        prune_interval,
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
            info!("Replaying indexer events from db at {}", db_path.display());
            IndexerState::new_without_genesis_events(state_config)?
        }
        InitializationMode::Sync => {
            info!("Syncing indexer state from db at {}", db_path.display());
            IndexerState::new_without_genesis_events(state_config)?
        }
    };

    if let Some(ref blocks_dir) = blocks_dir {
        match initialization_mode {
            InitializationMode::New => {
                let mut block_parser = match BlockParser::new_with_canonical_chain_discovery(
                    blocks_dir,
                    version,
                    canonical_threshold,
                    reporting_freq,
                ) {
                    Ok(block_parser) => block_parser,
                    Err(e) => {
                        panic!("Obtaining block parser failed: {}", e);
                    }
                };
                info!("Initializing indexer state");
                state.initialize_with_canonical_chain_discovery(&mut block_parser)?;
            }
            InitializationMode::Replay => {
                let min_length_filter = state.replay_events()?;
                let mut block_parser = BlockParser::new_length_sorted_min_filtered(
                    blocks_dir,
                    version,
                    min_length_filter,
                )?;

                if block_parser.total_num_blocks > 0 {
                    info!("Adding new blocks from {}", blocks_dir.display());
                    state.add_blocks(&mut block_parser)?;
                }
            }
            InitializationMode::Sync => {
                let min_length_filter = state.sync_from_db()?;
                let mut block_parser = BlockParser::new_length_sorted_min_filtered(
                    blocks_dir,
                    version,
                    min_length_filter,
                )?;

                if block_parser.total_num_blocks > 0 {
                    info!("Adding new blocks from {}", blocks_dir.display());
                    state.add_blocks(&mut block_parser)?;
                }
            }
        }
    } else {
        match initialization_mode {
            InitializationMode::New => (),
            InitializationMode::Replay => {
                state.replay_events()?;
            }
            InitializationMode::Sync => {
                state.sync_from_db()?;
            }
        }
    }

    staking_ledgers_dir
        .as_ref()
        .iter()
        .for_each(|d| state.add_startup_staking_ledgers_to_store(d).unwrap());
    Ok(state)
}

#[cfg(target_os = "linux")]
fn matches_event_kind(kind: EventKind) -> bool {
    matches!(
        kind,
        EventKind::Access(notify::event::AccessKind::Close(
            notify::event::AccessMode::Write
        )) | EventKind::Modify(notify::event::ModifyKind::Name(_))
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

pub async fn run(
    block_watch_dir: impl AsRef<Path>,
    staking_ledger_watch_dir: impl AsRef<Path>,
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
                    error!("Error sending event result: {}", e);
                }
            });
        },
        Config::default(),
    )?;

    watcher.watch(block_watch_dir.as_ref(), RecursiveMode::NonRecursive)?;
    info!(
        "Watching for new blocks in directory: {}",
        block_watch_dir.as_ref().display()
    );
    watcher.watch(
        staking_ledger_watch_dir.as_ref(),
        RecursiveMode::NonRecursive,
    )?;
    info!(
        "Watching for staking ledgers in directory: {}",
        staking_ledger_watch_dir.as_ref().display()
    );

    // watch for precomputed blocks & staking ledgers, and
    // recover missing blocks
    loop {
        tokio::select! {
            // watch for shutdown signal
            _ = wait_for_signal() => {
                info!("Ingestion shutdown signal received");
                break;
            }

            // watch for precomputed blocks & staking ledgers
            Some(res) = rx.recv() => {
                match res {
                    Ok(event) => process_event(event, &state).await?,
                    Err(e) => error!("Ingestion watcher error: {}", e),
                }
            }

            // recover any missing blocks
            _ = tokio::time::sleep(std::time::Duration::from_secs(missing_block_recovery_delay.unwrap_or(180))) => {
                if let Some(ref missing_block_recovery_exe) = missing_block_recovery_exe {
                    recover_missing_blocks(&state, &block_watch_dir, missing_block_recovery_exe, missing_block_recovery_batch).await
                }
            }
        }
    }

    info!("Ingestion cleanly shutdown");
    Ok(())
}

/// Precomputed block & staking ledger event handler
async fn process_event(event: Event, state: &Arc<RwLock<IndexerState>>) -> anyhow::Result<()> {
    trace!("Event: {event:?}");
    if matches_event_kind(event.kind) {
        for path in event.paths {
            if block::is_valid_block_file(&path) {
                debug!("Valid precomputed block file: {}", path.display());

                // TODO how to handle stop slots/blocks & final ledger?
                // Can we generate the new ledger & load the new network's config from a
                // specific block?

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
                            Err(e) => error!("Error adding block: {}", e),
                        }

                        // check for block parser version update
                        if state.version.chain_id.0 != *MAINNET_GENESIS_HASH {
                            trace!("Changing block parser from {}", version.version);
                            version.version.update()?;

                            trace!("Block parser changed to {}", version.version);
                            version.chain_id = state.version.chain_id.clone();
                        }
                    }
                    Err(e) => error!("Error parsing precomputed block: {}", e),
                }
            } else if staking::is_valid_ledger_file(&path) {
                // Acquire write lock
                let version = state.read().await.version.clone();
                let mut state = state.write().await;

                if let Some(store) = state.indexer_store.as_ref() {
                    match StakingLedger::parse_file(&path, version.version.clone()) {
                        Ok(staking_ledger) => {
                            let epoch = staking_ledger.epoch;
                            let ledger_hash = staking_ledger.ledger_hash.clone();
                            let ledger_summary = staking_ledger.summary();

                            match store.add_staking_ledger(staking_ledger) {
                                Ok(_) => {
                                    state.staking_ledgers.insert(epoch, ledger_hash);
                                    info!("Added staking ledger {}", ledger_summary);
                                }
                                Err(e) => {
                                    error!("Error adding staking ledger {} {}", ledger_summary, e)
                                }
                            }
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
                "Error recovery missing block: {}, pgm: {}, args: {:?}",
                e,
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
    pub fn new(network: &Network, chain_id: &ChainId) -> Self {
        Self {
            chain_id: chain_id.clone(),
            version: PcbVersion::V1,
            network: network.clone(),
            history: HashMap::from([(chain_id.clone(), PcbVersion::V1)]),
        }
    }

    pub fn new_testing() -> Self {
        Self::new(&Network::Mainnet, &ChainId("TESTING".into()))
    }
}

fn log_dirs_msg(blocks_dir: Option<&PathBuf>, staking_ledgers_dir: Option<&PathBuf>) {
    if let (Some(blocks_dir), Some(staking_ledgers_dir)) = (blocks_dir, staking_ledgers_dir) {
        info!(
            "Initializing indexer state from blocks in {} and staking ledgers in {}",
            blocks_dir.display(),
            staking_ledgers_dir.display(),
        );
    } else if let Some(blocks_dir) = blocks_dir.as_ref() {
        info!(
            "Initializing indexer state from blocks in {}",
            blocks_dir.display(),
        );
    } else if let Some(staking_ledgers_dir) = staking_ledgers_dir.as_ref() {
        info!(
            "Initializing indexer state from staking ledgers in {}",
            staking_ledgers_dir.display(),
        );
    }
}
