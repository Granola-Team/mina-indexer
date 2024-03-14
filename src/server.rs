use crate::{
    block::{self, parser::BlockParser, precomputed::PrecomputedBlock, BlockHash},
    constants::MAINNET_TRANSITION_FRONTIER_K,
    ledger::{
        genesis::GenesisLedger,
        staking::{self, StakingLedger},
        store::LedgerStore,
    },
    state::{IndexerState, IndexerStateConfig},
    store::IndexerStore,
    unix_socket_server::{self, UnixSocketServer},
};
use notify::{EventKind, RecursiveMode, Watcher};
use notify_debouncer_full::new_debouncer;
use std::{
    fs,
    path::{Path, PathBuf},
    process,
    sync::Arc,
    time::Duration,
};
use tokio::{
    runtime::Handle,
    sync::{mpsc, RwLock},
    task::JoinHandle,
};
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Clone, Debug)]
pub struct IndexerConfiguration {
    pub genesis_ledger: GenesisLedger,
    pub genesis_hash: BlockHash,
    pub block_startup_dir: PathBuf,
    pub block_watch_dir: PathBuf,
    pub ledger_startup_dir: PathBuf,
    pub ledger_watch_dir: PathBuf,
    pub prune_interval: u32,
    pub canonical_threshold: u32,
    pub canonical_update_threshold: u32,
    pub initialization_mode: InitializationMode,
    pub ledger_cadence: u32,
    pub reporting_freq: u32,
}

pub struct MinaIndexer {
    _witness_join_handle: JoinHandle<()>,
}

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
        let ledger_watch_dir = config.ledger_watch_dir.clone();

        let _witness_join_handle = tokio::spawn(async move {
            let state = match initialize(config, store).await {
                Ok(state) => state,
                Err(e) => {
                    error!("Error in server initialization: {e}");
                    std::process::exit(1);
                }
            };
            let state = Arc::new(RwLock::new(state));
            // Needs read-only state for summary
            unix_socket_server::start(UnixSocketServer::new(state.clone())).await;

            // This modifies the state
            if let Err(e) = run(block_watch_dir, ledger_watch_dir, state).await {
                error!("Error in server run: {}", e);
                std::process::exit(1);
            }
        });

        Ok(Self {
            _witness_join_handle,
        })
    }

    pub async fn await_loop(self) {
        let _ = self._witness_join_handle.await;
    }
}

async fn wait_for_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut term = signal(SignalKind::terminate()).expect("failed to register signal handler");
    let mut int = signal(SignalKind::interrupt()).expect("failed to register signal handler");
    tokio::select! {
        _ = term.recv() => {
            trace!("Received SIGTERM");
            process::exit(100);
        },
        _ = int.recv() => {
            info!("Received SIGINT");
            process::exit(101);
        },
    }
}

async fn setup_signal_handler() {
    tokio::spawn(async move {
        let _ = wait_for_signal().await;
    });
}

pub async fn initialize(
    config: IndexerConfiguration,
    store: Arc<IndexerStore>,
) -> anyhow::Result<IndexerState> {
    info!("Starting mina-indexer server");
    setup_signal_handler().await;

    let db_path = store.db_path.clone();
    let IndexerConfiguration {
        genesis_ledger,
        genesis_hash,
        block_startup_dir,
        ledger_startup_dir,
        prune_interval,
        canonical_threshold,
        canonical_update_threshold,
        initialization_mode,
        ledger_cadence,
        reporting_freq,
        ..
    } = config;

    fs::create_dir_all(block_startup_dir.clone())?;
    fs::create_dir_all(ledger_startup_dir.clone())?;

    let state_config = IndexerStateConfig {
        genesis_hash,
        genesis_ledger: genesis_ledger.clone(),
        indexer_store: store,
        transition_frontier_length: MAINNET_TRANSITION_FRONTIER_K,
        prune_interval,
        canonical_update_threshold,
        ledger_cadence,
        reporting_freq,
    };

    let mut state = match initialization_mode {
        InitializationMode::New => {
            info!(
                "Initializing indexer state from blocks in {} and staking ledgers in {}",
                block_startup_dir.display(),
                ledger_startup_dir.display(),
            );
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

    match initialization_mode {
        InitializationMode::New => {
            let mut block_parser = match BlockParser::new_with_canonical_chain_discovery(
                &block_startup_dir,
                canonical_threshold,
                reporting_freq,
            ) {
                Ok(block_parser) => block_parser,
                Err(e) => {
                    panic!("Obtaining block parser failed: {e}");
                }
            };
            info!("Initializing indexer state");
            state
                .initialize_with_canonical_chain_discovery(&mut block_parser)
                .await?;
            state.add_startup_staking_ledgers_to_store(&ledger_startup_dir)?;
        }
        InitializationMode::Replay => {
            let min_length_filter = state.replay_events()?;
            let mut block_parser =
                BlockParser::new_length_sorted_min_filtered(&block_startup_dir, min_length_filter)?;
            state.add_blocks(&mut block_parser).await?;
            state.add_startup_staking_ledgers_to_store(&ledger_startup_dir)?;
        }
        InitializationMode::Sync => {
            let min_length_filter = state.sync_from_db()?;
            let mut block_parser =
                BlockParser::new_length_sorted_min_filtered(&block_startup_dir, min_length_filter)?;
            state.add_blocks(&mut block_parser).await?;
            state.add_startup_staking_ledgers_to_store(&ledger_startup_dir)?;
        }
    }
    Ok(state)
}

#[instrument(skip_all)]
pub async fn run(
    block_watch_dir: impl AsRef<Path>,
    ledger_watch_dir: impl AsRef<Path>,
    state: Arc<RwLock<IndexerState>>,
) -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel(100);
    let rt = Handle::current();

    let mut debouncer = new_debouncer(Duration::from_secs(1), None, move |result| {
        let tx = tx.clone();
        rt.spawn(async move {
            if let Err(e) = tx.send(result).await {
                error!("Error sending event result: {:?}", e);
            }
        });
    })
    .unwrap();

    debouncer
        .watcher()
        .watch(block_watch_dir.as_ref(), RecursiveMode::NonRecursive)?;
    debouncer
        .watcher()
        .watch(ledger_watch_dir.as_ref(), RecursiveMode::NonRecursive)?;

    debouncer
        .cache()
        .add_root(block_watch_dir.as_ref(), RecursiveMode::Recursive);
    debouncer
        .cache()
        .add_root(ledger_watch_dir.as_ref(), RecursiveMode::Recursive);

    info!(
        "Watching for new blocks in directory: {:?}",
        block_watch_dir.as_ref()
    );
    info!(
        "Watching for staking ledgers in directory: {:?}",
        ledger_watch_dir.as_ref()
    );

    while let Some(res) = rx.recv().await {
        match res {
            Ok(events) => {
                for d_event in events {
                    let event = d_event.event;
                    if let EventKind::Create(notify::event::CreateKind::File)
                    | EventKind::Modify(notify::event::ModifyKind::Name(_)) = event.kind
                    {
                        for path in event.paths {
                            if block::is_valid_block_file(&path) {
                                debug!("Valid precomputed block file: {}", path.display());
                                if let Ok(block) = PrecomputedBlock::parse_file(&path) {
                                    let mut state = state.write().await;
                                    if state.block_pipeline(&block).is_ok() {
                                        info!(
                                            "Added block (length {}): {}",
                                            block.blockchain_length, block.state_hash
                                        );
                                    }
                                } else {
                                    warn!("Unable to parse precomputed block: {path:?}");
                                }
                            } else if staking::is_valid_ledger_file(&path) {
                                let state = state.write().await;
                                if let Some(store) = state.indexer_store.as_ref() {
                                    if let Ok(staking_ledger) = StakingLedger::parse_file(&path) {
                                        let ledger_hash = staking_ledger.ledger_hash.clone();
                                        match store.add_staking_ledger(staking_ledger) {
                                            Ok(_) => {
                                                info!("Added staking ledger {:?}", ledger_hash);
                                            }
                                            Err(e) => error!("Error adding staking ledger: {}", e),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("Block watcher error: {e:?}");
            }
        }
    }
    Ok(())
}
