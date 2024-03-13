use crate::{
    block::{parser::BlockParser, BlockHash},
    constants::{MAINNET_TRANSITION_FRONTIER_K, SOCKET_NAME},
    ipc::IpcActor,
    ledger::{genesis::GenesisLedger, staking::parser::StakingLedgerParser, store::LedgerStore},
    receiver::{filesystem::FilesystemReceiver, Receiver},
    state::{summary::SummaryVerbose, IndexerState, IndexerStateConfig},
    store::IndexerStore,
};
use interprocess::local_socket::tokio::LocalSocketListener;
use std::{
    fs,
    path::{Path, PathBuf},
    process,
    sync::Arc,
};
use tokio::{io, sync::mpsc, task::JoinHandle};
use tracing::{debug, error, info, instrument, trace};

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
    _ipc_join_handle: JoinHandle<()>,
    _witness_join_handle: JoinHandle<()>,
}

#[derive(Debug)]
pub struct IpcChannelUpdate {
    pub summary: Box<SummaryVerbose>,
    pub store: Arc<IndexerStore>,
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
        let (ipc_update_sender, ipc_update_receiver) = mpsc::channel::<IpcChannelUpdate>(1);
        let ipc_update_arc = Arc::new(ipc_update_sender);
        let block_watch_dir = config.block_watch_dir.clone();
        let ledger_watch_dir = config.ledger_watch_dir.clone();
        let ipc_store = store.clone();

        let listener = LocalSocketListener::bind(SOCKET_NAME)
            .or_else(try_remove_old_socket)
            .unwrap_or_else(|e| panic!("unable to connect to domain socket: {e}"));

        debug!("Local socket listener started");

        let _ipc_join_handle = tokio::spawn(async move {
            debug!("Spawning IPC Actor");

            let mut ipc_actor = IpcActor::new(listener, ipc_store, ipc_update_receiver);
            ipc_actor.run().await
        });
        let _witness_join_handle = tokio::spawn(async move {
            let state = match initialize(config, store, ipc_update_arc.clone()).await {
                Ok(state) => state,
                Err(e) => {
                    error!("Error in server initialization: {e}");
                    std::process::exit(1);
                }
            };
            if let Err(e) = run(
                block_watch_dir,
                ledger_watch_dir,
                state,
                ipc_update_arc.clone(),
            )
            .await
            {
                error!("Error in server run: {e}");
                std::process::exit(1);
            }
        });

        Ok(Self {
            _ipc_join_handle,
            _witness_join_handle,
        })
    }

    pub async fn await_loop(self) {
        let _ = self._witness_join_handle.await;
        let _ = self._ipc_join_handle.await;
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
    ipc_update_sender: Arc<mpsc::Sender<IpcChannelUpdate>>,
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

    let summary = Box::new(state.summary_verbose());
    let store = Arc::new(state.spawn_secondary_database()?);

    debug!("Updating IPC state");
    ipc_update_sender
        .send(IpcChannelUpdate { summary, store })
        .await?;

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

    ipc_update_sender
        .send(IpcChannelUpdate {
            summary: Box::new(state.summary_verbose()),
            store: Arc::new(state.spawn_secondary_database()?),
        })
        .await?;

    Ok(state)
}

#[instrument(skip_all)]
pub async fn run(
    block_watch_dir: impl AsRef<Path>,
    ledger_watch_dir: impl AsRef<Path>,
    mut state: IndexerState,
    ipc_update_sender: Arc<mpsc::Sender<IpcChannelUpdate>>,
) -> anyhow::Result<()> {
    let mut fs_block_receiver = <FilesystemReceiver<BlockParser>>::new().await?;
    fs_block_receiver.load_directory(block_watch_dir.as_ref())?;
    info!(
        "Block receiver set to watch {}",
        block_watch_dir.as_ref().to_path_buf().display()
    );

    let mut fs_ledger_receiver = <FilesystemReceiver<StakingLedgerParser>>::new().await?;
    fs_ledger_receiver.load_directory(ledger_watch_dir.as_ref())?;
    info!(
        "Staking ledger receiver set to watch {}",
        ledger_watch_dir.as_ref().to_path_buf().display()
    );

    loop {
        tokio::select! {
            block_fut = fs_block_receiver.recv_data() => {
                match block_fut {
                    Ok(block_opt) => {
                        if let Some(precomputed_block) = block_opt {
                            debug!("Receiving block {}", precomputed_block.summary());

                            if state.block_pipeline(&precomputed_block)? {
                                info!("Added block {}", precomputed_block.summary());
                            }

                            ipc_update_sender.send(IpcChannelUpdate {
                                summary: Box::new(state.summary_verbose()),
                                store: Arc::new(state.spawn_secondary_database()?),
                            }).await?;
                        } else {
                            error!("Block receiver channel closed");
                        }
                    }
                    Err(e) => {
                        error!("Unable to receive block: {e}");
                    }
                }
            }

            staking_ledger_fut = fs_ledger_receiver.recv_data() => {
                match staking_ledger_fut {
                    Ok(ledger_opt) => {
                        if let Some(staking_ledger) = ledger_opt {
                            debug!("Receiving staking ledger {}", staking_ledger.summary());

                            if let Some(ref store) = state.indexer_store {
                                let ledger = staking_ledger.summary();
                                match store.add_staking_ledger(staking_ledger) {
                                    Ok(_) => info!("Added staking ledger {ledger}"),
                                    Err(e) => error!("Error adding staking ledger: {e}"),
                                }
                            }

                            ipc_update_sender.send(IpcChannelUpdate {
                                summary: Box::new(state.summary_verbose()),
                                store: Arc::new(state.spawn_secondary_database()?),
                            }).await?;
                        } else {
                            error!("Staking ledger receiver closed channel");
                        }
                    }
                    Err(e) => {
                        error!("Unable to receive staking ledger: {e}");
                    }
                }
            }
        }
    }
}

fn try_remove_old_socket(e: io::Error) -> io::Result<LocalSocketListener> {
    if e.kind() == io::ErrorKind::AddrInUse {
        debug!(
            "Domain socket: {} already in use. Removing old vestige",
            &SOCKET_NAME
        );
        remove_domain_socket()?;
        LocalSocketListener::bind(SOCKET_NAME)
    } else {
        Err(e)
    }
}

pub fn remove_domain_socket() -> io::Result<()> {
    std::fs::remove_file(SOCKET_NAME)?;
    debug!("Domain socket removed: {SOCKET_NAME}");
    Ok(())
}
