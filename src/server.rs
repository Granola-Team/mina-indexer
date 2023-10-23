use crate::{
    block::{parser::BlockParser, Block, BlockHash, BlockWithoutHeight},
    ipc::IpcActor,
    receiver::{filesystem::FilesystemReceiver, BlockReceiver},
    state::{
        ledger::{genesis::GenesisRoot, Ledger},
        summary::SummaryVerbose,
        IndexerState, Tip,
    },
    store::IndexerStore,
    MAINNET_TRANSITION_FRONTIER_K, SOCKET_NAME,
};
use anyhow::anyhow;
use interprocess::local_socket::tokio::LocalSocketListener;

use serde_derive::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process,
    sync::Arc,
    time::Duration,
};
use tokio::{
    fs::{create_dir_all, metadata},
    io,
    sync::{
        mpsc::{self, Sender},
        watch,
    },
    task::JoinHandle,
};
use tracing::{debug, info, instrument};

#[derive(Clone, Debug)]
pub struct IndexerConfiguration {
    pub ledger: GenesisRoot,
    pub is_genesis_ledger: bool,
    pub root_hash: BlockHash,
    pub startup_dir: PathBuf,
    pub watch_dir: PathBuf,
    pub prune_interval: u32,
    pub canonical_threshold: u32,
    pub canonical_update_threshold: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct SaveCommand(PathBuf);

#[derive(Debug, Serialize, Deserialize)]
struct SaveResponse(String);

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MinaIndexerRunPhase {
    JustStarted,
    ConnectingToIPCSocket,
    SettingSIGINTHandler,
    InitializingState,
    StateInitializedFromParser,
    StartingBlockReceiver,
    StartingIPCSocketListener,
    StartingMainServerLoop,
    ReceivingBlock,
    ReceivingIPCConnection,
    SavingStateSnapshot,
}

pub enum MinaIndexerQuery {
    NumBlocksProcessed,
    BestTip,
    CanonicalTip,
    Uptime,
}

pub enum MinaIndexerQueryResponse {
    NumBlocksProcessed(u32),
    BestTip(Tip),
    CanonicalTip(Tip),
    Uptime(Duration),
}

pub struct MinaIndexer {
    _loop_join_handle: JoinHandle<anyhow::Result<()>>,
    phase_receiver: watch::Receiver<MinaIndexerRunPhase>,
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

impl MinaIndexer {
    pub async fn new(
        config: IndexerConfiguration,
        store: Arc<IndexerStore>,
    ) -> anyhow::Result<Self> {
        let (phase_sender, phase_receiver) = watch::channel(MinaIndexerRunPhase::JustStarted);
        let phase_sender = Arc::new(phase_sender);
        let (query_sender, query_receiver) = mpsc::unbounded_channel();
        let watch_dir = config.watch_dir.clone();
        let config_new = config.clone();
        let (ipc_update_sender, ipc_update_receiver) = mpsc::channel::<IpcChannelUpdate>(1);
        let ipc_store = store.clone();
        let ipc_update_arc = Arc::new(ipc_update_sender.clone());

        let _loop_join_handle = tokio::spawn(async move {
            use MinaIndexerRunPhase::*;
            phase_sender.send_replace(StartingIPCSocketListener);
            // LocalSocketStream::connect(SOCKET_NAME)
            //     .await
            //     .expect_err("Server is already running... Exiting.");
            let listener = LocalSocketListener::bind(SOCKET_NAME).unwrap_or_else(|e| {
                if e.kind() == io::ErrorKind::AddrInUse {
                    let name = &SOCKET_NAME;
                    debug!(
                        "Domain socket: {} already in use. Removing old vestige",
                        name
                    );
                    std::fs::remove_file(name).expect("Should be able to remove socket file");
                    LocalSocketListener::bind(SOCKET_NAME).unwrap_or_else(|e| {
                        panic!("Unable to bind domain socket {:?}", e);
                    })
                } else {
                    panic!("Unable to bind domain socket {:?}", e);
                }
            });
            info!("Local socket listener started");
            tokio::spawn(async move {
                let mut ipc_actor =
                    IpcActor::new(config_new, listener, ipc_store, ipc_update_receiver);
                info!("Spawning IPC Actor");
                ipc_actor.run().await
            });
            let (state, phase_sender) =
                initialize(config, store, phase_sender, ipc_update_arc.clone()).await?;
            run(
                watch_dir,
                state,
                phase_sender,
                query_receiver,
                ipc_update_arc.clone(),
            )
            .await
        });

        Ok(Self {
            _loop_join_handle,
            phase_receiver,
            query_sender,
            _ipc_update_sender: ipc_update_sender,
        })
    }

    pub async fn await_loop(self) {
        let _ = self._loop_join_handle.await;
    }

    async fn send_query(
        &self,
        command: MinaIndexerQuery,
    ) -> anyhow::Result<MinaIndexerQueryResponse> {
        let (response_sender, response_receiver) = oneshot::channel();
        self.query_sender
            .send((command, response_sender))
            .map_err(|_| anyhow!("could not send command to running Mina Indexer"))?;
        response_receiver.recv().map_err(|recv_err| recv_err.into())
    }

    pub fn initialized(&self) -> bool {
        use MinaIndexerRunPhase::*;
        !matches!(
            *self.phase_receiver.borrow(),
            JustStarted | SettingSIGINTHandler | InitializingState
        )
    }

    pub fn state(&self) -> MinaIndexerRunPhase {
        *self.phase_receiver.borrow()
    }

    pub async fn blocks_processed(&self) -> anyhow::Result<u32> {
        match self
            .send_query(MinaIndexerQuery::NumBlocksProcessed)
            .await?
        {
            MinaIndexerQueryResponse::NumBlocksProcessed(blocks_processed) => Ok(blocks_processed),
            _ => Err(anyhow!("unexpected response!")),
        }
    }
}

pub async fn initialize(
    config: IndexerConfiguration,
    store: Arc<IndexerStore>,
    phase_sender: Arc<watch::Sender<MinaIndexerRunPhase>>,
    ipc_update_sender: Arc<mpsc::Sender<IpcChannelUpdate>>,
) -> anyhow::Result<(IndexerState, Arc<watch::Sender<MinaIndexerRunPhase>>)> {
    use MinaIndexerRunPhase::*;

    phase_sender.send_replace(SettingSIGINTHandler);
    debug!("Setting Ctrl-C handler");
    ctrlc::set_handler(move || {
        info!("SIGINT received. Exiting.");
        process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    phase_sender.send_replace(InitializingState);
    info!("Starting mina-indexer server");
    let IndexerConfiguration {
        ledger,
        is_genesis_ledger,
        root_hash,
        startup_dir,
        watch_dir: _,
        prune_interval,
        canonical_threshold,
        canonical_update_threshold,
    } = config;

    create_dir_if_non_existent(startup_dir.to_str().unwrap()).await;

    let state = {
        info!(
            "Initializing indexer state from blocks in {}",
            startup_dir.display()
        );
        let mut state = IndexerState::new(
            root_hash.clone(),
            ledger.ledger.clone(),
            store,
            MAINNET_TRANSITION_FRONTIER_K,
            prune_interval,
            canonical_update_threshold,
        )?;

        info!("Getting best tip");
        let best_tip = state.best_tip_block().clone();
        info!("Getting best ledger");
        let ledger = ledger.ledger.into();
        info!("Getting summary");
        let summary = Box::new(state.summary_verbose());
        info!("Getting store");
        let store = Arc::new(state.spawn_secondary_database()?);
        info!("Updating IPC state");
        ipc_update_sender
            .send(IpcChannelUpdate {
                best_tip,
                ledger,
                summary,
                store,
            })
            .await?;

        info!("Parsing blocks");
        let mut block_parser = BlockParser::new(&startup_dir, canonical_threshold)?;
        if is_genesis_ledger {
            state
                .initialize_with_contiguous_canonical(&mut block_parser)
                .await?;
        } else {
            state
                .initialize_without_contiguous_canonical(&mut block_parser)
                .await?;
        }

        phase_sender.send_replace(StateInitializedFromParser);
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

    Ok((state, phase_sender))
}

#[instrument(skip_all)]
pub async fn run(
    block_watch_dir: impl AsRef<Path>,
    mut state: IndexerState,
    phase_sender: Arc<watch::Sender<MinaIndexerRunPhase>>,
    mut query_receiver: mpsc::UnboundedReceiver<(
        MinaIndexerQuery,
        oneshot::Sender<MinaIndexerQueryResponse>,
    )>,
    ipc_update_sender: Arc<mpsc::Sender<IpcChannelUpdate>>,
) -> Result<(), anyhow::Error> {
    use MinaIndexerRunPhase::*;

    phase_sender.send_replace(StartingBlockReceiver);
    let mut filesystem_receiver = FilesystemReceiver::new(1024, 64).await?;
    filesystem_receiver.load_directory(block_watch_dir.as_ref())?;
    info!("Block receiver set to watch {:?}", block_watch_dir.as_ref());

    phase_sender.send_replace(StartingMainServerLoop);

    loop {
        tokio::select! {
            Some((command, response_sender)) = query_receiver.recv() => {
                use MinaIndexerQuery::*;
                let response = match command {
                    NumBlocksProcessed
                        => MinaIndexerQueryResponse::NumBlocksProcessed(state.blocks_processed),
                    BestTip => {
                        let best_tip = state.best_tip.clone();
                        MinaIndexerQueryResponse::BestTip(best_tip)
                    },
                    CanonicalTip => {
                        let canonical_tip = state.canonical_tip.clone();
                        MinaIndexerQueryResponse::CanonicalTip(canonical_tip)
                    },
                    Uptime
                        => MinaIndexerQueryResponse::Uptime(state.init_time.elapsed())
                };
                response_sender.send(response)?;
            }

            block_fut = filesystem_receiver.recv_block() => {
                phase_sender.send_replace(ReceivingBlock);
                if let Some(precomputed_block) = block_fut? {
                    let block = BlockWithoutHeight::from_precomputed(&precomputed_block);
                    debug!("Receiving block {block:?}");

                    state.add_block(&precomputed_block)?;
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

pub async fn create_dir_if_non_existent(path: &str) {
    if metadata(path).await.is_err() {
        debug!("Creating directory {path:?}");
        create_dir_all(path).await.unwrap();
    }
}

pub async fn spawn_readonly_rocksdb(primary_path: PathBuf) -> anyhow::Result<IndexerStore> {
    let mut secondary_path = primary_path.clone();
    secondary_path.push(uuid::Uuid::new_v4().to_string());

    debug!("Spawning secondary readonly RocksDB instance");
    let block_store_readonly = IndexerStore::new_read_only(&primary_path, &secondary_path)?;
    Ok(block_store_readonly)
}
