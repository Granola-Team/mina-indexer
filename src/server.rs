use crate::{
    block::{parser::BlockParser, Block, BlockHash, BlockWithoutHeight},
    constants::{MAINNET_TRANSITION_FRONTIER_K, SOCKET_NAME},
    ipc::IpcActor,
    ledger::{genesis::GenesisRoot, Ledger},
    receiver::{filesystem::FilesystemReceiver, BlockReceiver},
    state::{summary::SummaryVerbose, IndexerState},
    store::IndexerStore,
};
use anyhow::anyhow;
use interprocess::local_socket::tokio::LocalSocketListener;
use std::{
    fs,
    path::{Path, PathBuf},
    process,
    sync::Arc,
    time::Duration,
};
use tokio::{
    io,
    sync::mpsc::{self, Sender},
    task::JoinHandle,
};
use tracing::{debug, info, instrument};

#[derive(Clone, Debug)]
pub struct IndexerConfiguration {
    pub ledger: GenesisRoot,
    pub root_hash: BlockHash,
    pub startup_dir: PathBuf,
    pub watch_dir: PathBuf,
    pub prune_interval: u32,
    pub canonical_threshold: u32,
    pub canonical_update_threshold: u32,
    pub initialization_mode: InitializationMode,
}

pub enum MinaIndexerQuery {
    NumBlocksProcessed,
    BestTip,
    CanonicalTip,
    Uptime,
}

pub enum MinaIndexerQueryResponse {
    NumBlocksProcessed(u32),
    BestTip(String),
    CanonicalTip(String),
    Uptime(Duration),
}

pub struct MinaIndexer {
    _ipc_join_handle: JoinHandle<()>,
    _witness_join_handle: JoinHandle<anyhow::Result<()>>,
    query_sender:
        mpsc::UnboundedSender<(MinaIndexerQuery, oneshot::Sender<MinaIndexerQueryResponse>)>,
    _ipc_update_sender: Sender<IpcChannelUpdate>,
}

#[derive(Debug)]
pub struct IpcChannelUpdate {
    pub best_tip: Block,
    pub ledger: Ledger,
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
        let (query_sender, query_receiver) = mpsc::unbounded_channel();
        let (ipc_update_sender, ipc_update_receiver) = mpsc::channel::<IpcChannelUpdate>(1);
        let ipc_update_arc = Arc::new(ipc_update_sender.clone());
        let watch_dir = config.watch_dir.clone();
        let ipc_store = store.clone();
        let ipc_config = config.clone();
        let listener = LocalSocketListener::bind(SOCKET_NAME)
            .or_else(try_remove_old_socket)
            .unwrap_or_else(|e| panic!("unable to connect to domain socket: {:?}", e.to_string()));

        debug!("Local socket listener started");

        let _ipc_join_handle = tokio::spawn(async move {
            debug!("Spawning IPC Actor");

            let mut ipc_actor = IpcActor::new(ipc_config, listener, ipc_store, ipc_update_receiver);
            ipc_actor.run().await
        });
        let _witness_join_handle = tokio::spawn(async move {
            let state = initialize(config, store, ipc_update_arc.clone()).await?;
            run(watch_dir, state, query_receiver, ipc_update_arc.clone()).await
        });

        Ok(Self {
            _ipc_join_handle,
            _witness_join_handle,
            query_sender,
            _ipc_update_sender: ipc_update_sender,
        })
    }

    pub async fn await_loop(self) {
        let _ = self._witness_join_handle.await;
        let _ = self._ipc_join_handle.await;
    }

    fn send_query(&self, command: MinaIndexerQuery) -> anyhow::Result<MinaIndexerQueryResponse> {
        let (response_sender, response_receiver) = oneshot::channel();
        self.query_sender
            .send((command, response_sender))
            .map_err(|_| anyhow!("could not send command to running Mina Indexer"))?;
        response_receiver.recv().map_err(|recv_err| recv_err.into())
    }

    pub fn blocks_processed(&self) -> anyhow::Result<u32> {
        match self.send_query(MinaIndexerQuery::NumBlocksProcessed)? {
            MinaIndexerQueryResponse::NumBlocksProcessed(blocks_processed) => Ok(blocks_processed),
            _ => Err(anyhow!("unexpected response!")),
        }
    }
}

pub async fn initialize(
    config: IndexerConfiguration,
    store: Arc<IndexerStore>,
    ipc_update_sender: Arc<mpsc::Sender<IpcChannelUpdate>>,
) -> anyhow::Result<IndexerState> {
    debug!("Setting Ctrl-C handler");
    ctrlc::set_handler(move || {
        info!("SIGINT received. Exiting.");
        process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    info!("Starting mina-indexer server");
    let db_path = store.db_path.clone();
    let IndexerConfiguration {
        ledger,
        root_hash,
        startup_dir,
        watch_dir: _,
        prune_interval,
        canonical_threshold,
        canonical_update_threshold,
        initialization_mode,
    } = config;

    fs::create_dir_all(startup_dir.clone()).expect("startup_dir created");

    let state = {
        let mut state = match initialization_mode {
            InitializationMode::New => {
                info!(
                    "Initializing indexer state from blocks in {}",
                    startup_dir.display()
                );
                IndexerState::new(
                    &root_hash,
                    ledger.ledger.clone(),
                    store,
                    MAINNET_TRANSITION_FRONTIER_K,
                    prune_interval,
                    canonical_update_threshold,
                )?
            }
            InitializationMode::Replay => {
                info!("Replaying indexer events from db at {}", db_path.display());
                IndexerState::new_without_genesis_events(
                    &root_hash,
                    ledger.ledger.clone(),
                    store,
                    MAINNET_TRANSITION_FRONTIER_K,
                    prune_interval,
                    canonical_update_threshold,
                )?
            }
            InitializationMode::Sync => {
                info!("Syncing indexer state from db at {}", db_path.display());
                IndexerState::new_without_genesis_events(
                    &root_hash,
                    ledger.ledger.clone(),
                    store,
                    MAINNET_TRANSITION_FRONTIER_K,
                    prune_interval,
                    canonical_update_threshold,
                )?
            }
        };

        info!("Getting best tip");
        let best_tip = state.best_tip_block().clone();
        info!("Getting best ledger");
        let ledger = ledger.ledger.into();
        info!("Getting summary");
        let summary = Box::new(state.summary_verbose());
        info!("Getting store");
        let store = Arc::new(state.spawn_secondary_database()?);

        debug!("Updating IPC state");
        ipc_update_sender
            .send(IpcChannelUpdate {
                best_tip,
                ledger,
                summary,
                store,
            })
            .await?;

        match initialization_mode {
            InitializationMode::New => {
                info!("Parsing blocks");
                let mut block_parser = BlockParser::new(&startup_dir, canonical_threshold)?;
                state
                    .initialize_with_canonical_chain_discovery(&mut block_parser)
                    .await?;
            }
            InitializationMode::Replay => {
                state.replay_events()?;
            }
            InitializationMode::Sync => {
                state.sync_from_db()?;
            }
        }

        ipc_update_sender
            .send(IpcChannelUpdate {
                best_tip: state.best_tip_block().clone(),
                ledger: state.best_ledger()?.unwrap(),
                summary: Box::new(state.summary_verbose()),
                store: Arc::new(state.spawn_secondary_database()?),
            })
            .await?;
        state
    };

    Ok(state)
}

#[instrument(skip_all)]
pub async fn run(
    block_watch_dir: impl AsRef<Path>,
    mut state: IndexerState,
    mut query_receiver: mpsc::UnboundedReceiver<(
        MinaIndexerQuery,
        oneshot::Sender<MinaIndexerQueryResponse>,
    )>,
    ipc_update_sender: Arc<mpsc::Sender<IpcChannelUpdate>>,
) -> Result<(), anyhow::Error> {
    let mut filesystem_receiver = FilesystemReceiver::new(1024, 64).await?;
    filesystem_receiver.load_directory(block_watch_dir.as_ref())?;
    info!(
        "Block receiver set to watch {}",
        block_watch_dir.as_ref().to_path_buf().display()
    );

    loop {
        tokio::select! {
            Some((command, response_sender)) = query_receiver.recv() => {
                use MinaIndexerQuery::*;
                let response = match command {
                    NumBlocksProcessed
                        => MinaIndexerQueryResponse::NumBlocksProcessed(state.blocks_processed),
                    BestTip => {
                        let best_tip = state.best_tip.clone().state_hash.0;
                        MinaIndexerQueryResponse::BestTip(best_tip)
                    },
                    CanonicalTip => {
                        let canonical_tip = state.canonical_tip.clone().state_hash.0;
                        MinaIndexerQueryResponse::CanonicalTip(canonical_tip)
                    },
                    Uptime
                        => MinaIndexerQueryResponse::Uptime(state.init_time.elapsed())
                };
                response_sender.send(response)?;
            }

            block_fut = filesystem_receiver.recv_block() => {
                if let Some(precomputed_block) = block_fut? {
                    let block = BlockWithoutHeight::from_precomputed(&precomputed_block);
                    debug!("Receiving block {block:?}");

                    state.add_block_to_witness_tree(&precomputed_block)?;
                    info!("Added {block:?}");

                    ipc_update_sender.send(IpcChannelUpdate {
                        best_tip: state.best_tip_block().clone(),
                        ledger: state.best_ledger()?.unwrap(),
                        summary: Box::new(state.summary_verbose()),
                        store: Arc::new(state.spawn_secondary_database()?),
                    }).await?;
                } else {
                    info!("Block receiver shutdown, system exit");
                    return Ok(())
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
        std::fs::remove_file(SOCKET_NAME).expect("Should be able to remove socket file");
        LocalSocketListener::bind(SOCKET_NAME)
    } else {
        Err(e)
    }
}
