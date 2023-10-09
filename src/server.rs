use crate::{
    block::{parser::BlockParser, store::BlockStore, Block, BlockHash, BlockWithoutHeight},
    receiver::{filesystem::FilesystemReceiver, BlockReceiver},
    state::{
        ledger::{genesis::GenesisRoot, public_key::PublicKey, Ledger},
        summary::{SummaryShort, SummaryVerbose},
        IndexerState, Tip
    },
    store::IndexerStore,
    MAINNET_TRANSITION_FRONTIER_K, SOCKET_NAME,
};
use anyhow::anyhow;
use futures::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use interprocess::local_socket::tokio::{LocalSocketListener, LocalSocketStream};
use log::trace;

use serde_derive::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process,
    sync::Arc,
    time::Duration, cell::RefCell
};
use tokio::{
    fs::{self, create_dir_all, metadata},
    io,
    sync::{mpsc, watch},
    task::JoinHandle,
};
use tracing::{debug, error, info, instrument};

#[derive(Clone)]
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
    query_sender: mpsc::Sender<(MinaIndexerQuery, oneshot::Sender<MinaIndexerQueryResponse>)>,
}

pub enum IpcChannelUpdate {
    NewState {
        best_tip: Block,
        ledger: Ledger,
        summary: SummaryVerbose,
    },
    NewStore {
        store: Arc<IndexerStore>,
    }
}

pub async fn ipc_handler(
    config: IndexerConfiguration, 
    mut ipc_update_receiver: mpsc::Receiver<IpcChannelUpdate>, 
    listener: LocalSocketListener,
) {
    let best_tip = RefCell::new(Block { 
        parent_hash: config.root_hash.clone(), 
        state_hash: config.root_hash, 
        height: 1, 
        blockchain_length: 1, 
        global_slot_since_genesis: 0 
    });
    let summary = RefCell::new(None);
    let ledger: RefCell<Ledger> = RefCell::new(config.ledger.ledger.into());
    let block_store_readonly: RefCell<Option<Arc<IndexerStore>>> = RefCell::new(None);
    loop {
        tokio::select! {
            ipc_update = ipc_update_receiver.recv() => {
                match ipc_update {
                    Some(ipc_update) => match ipc_update {
                        IpcChannelUpdate::NewState { best_tip: new_best_tip, ledger: new_ledger, summary: new_summary } => {
                            *best_tip.borrow_mut() = new_best_tip;
                            *ledger.borrow_mut() = new_ledger;
                            *summary.borrow_mut() = Some(new_summary);
                        },
                        IpcChannelUpdate::NewStore { store } => {
                            *block_store_readonly.borrow_mut() = Some(store);
                        },
                    },
                    None => continue,
                }
            }

            conn_fut = listener.accept() => {
                match conn_fut {
                    Ok(stream) => {
                        info!("Accepted client connection");
                        // handle the connection
                        if let Some(block_store) = block_store_readonly.borrow().as_ref() {
                            info!("found block store");
                            let block_store = block_store.clone();
                            let ledger = ledger.borrow().clone();
                            let summary = summary.borrow().clone();
                            let best_tip = best_tip.borrow().clone();
                            tokio::spawn(async move {
                                info!("handling connection");
                                debug!("Handling client connection");
                                match handle_conn(stream, block_store.clone(), &best_tip, ledger, summary).await {
                                    Err(e) => {
                                        info!("error {e}");
                                        error!("Error handling connection: {e}");
                                    },
                                    Ok(_) => { info!("handled connection"); },
                                };
                                debug!("Removing readonly instance at {}", block_store.db_path.clone().display());
                                tokio::fs::remove_dir_all(&block_store.db_path).await.ok();
                            });
                        } else {
                            info!("No store was found :(");
                        }
                    }
                    Err(e) => {
                        error!("Error accepting connection: {}", e);
                    }
                }
            }
        
        }
    }
}

impl MinaIndexer {
    pub async fn new(
        config: IndexerConfiguration,
        store: Arc<IndexerStore>,
    ) -> anyhow::Result<Self> {
        let (phase_sender, phase_receiver) = watch::channel(MinaIndexerRunPhase::JustStarted);
        let phase_sender = Arc::new(phase_sender);

        let (query_sender, query_receiver) = mpsc::channel(1);

        let _loop_join_handle = tokio::spawn(async move {
            use MinaIndexerRunPhase::*;
            phase_sender.send_replace(StartingIPCSocketListener);
            // LocalSocketStream::connect(SOCKET_NAME)
            //     .await
            //     .expect_err("Server is already running... Exiting.");
            let (ipc_update_sender, ipc_update_receiver) = mpsc::channel::<IpcChannelUpdate>(50);
            let listener = LocalSocketListener::bind(SOCKET_NAME).unwrap_or_else(|e| {
                if e.kind() == io::ErrorKind::AddrInUse {
                    let name = &SOCKET_NAME[1..];
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
            let watch_dir = config.watch_dir.clone();
            let config_new = config.clone();
            tokio::spawn(async move { ipc_handler(config_new, ipc_update_receiver, listener).await;});
            let (state, phase_sender) = initialize(config, store, phase_sender, ipc_update_sender.clone()).await?;
            run(watch_dir, state, phase_sender, query_receiver, ipc_update_sender).await
        });

        Ok(Self {
            _loop_join_handle,
            phase_receiver,
            query_sender,
        })
    }

    async fn send_query(
        &self,
        command: MinaIndexerQuery,
    ) -> anyhow::Result<MinaIndexerQueryResponse> {
        let (response_sender, response_receiver) = oneshot::channel();
        self.query_sender
            .send((command, response_sender))
            .await
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
    ipc_update_sender: mpsc::Sender<IpcChannelUpdate>,
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

    let state = {
        info!(
            "Initializing indexer state from blocks in {}",
            startup_dir.display()
        );
        let mut state = IndexerState::new(
            root_hash.clone(),
            ledger.ledger,
            store,
            MAINNET_TRANSITION_FRONTIER_K,
            prune_interval,
            canonical_update_threshold,
        )?;

        ipc_update_sender.send(IpcChannelUpdate::NewStore { store: Arc::new(state.spawn_secondary_database()?) }).await?;
        ipc_update_sender.send(IpcChannelUpdate::NewState { best_tip: state.best_tip_block().clone(), ledger: state.best_ledger()?.unwrap(), summary: state.summary_verbose() }).await?;

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
        ipc_update_sender.send(IpcChannelUpdate::NewState { best_tip: state.best_tip_block().clone(), ledger: state.best_ledger()?.unwrap(), summary: state.summary_verbose() }).await?;
        ipc_update_sender.send(IpcChannelUpdate::NewStore { store: Arc::new(state.spawn_secondary_database()?) }).await?;
        state
    };

    Ok((state, phase_sender))
}

#[instrument(skip_all)]
pub async fn run(
    block_watch_dir: impl AsRef<Path>,
    mut state: IndexerState,
    phase_sender: Arc<watch::Sender<MinaIndexerRunPhase>>,
    mut query_receiver: mpsc::Receiver<(
        MinaIndexerQuery,
        oneshot::Sender<MinaIndexerQueryResponse>,
    )>,
    ipc_update_sender: mpsc::Sender<IpcChannelUpdate>
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

                    ipc_update_sender.send(IpcChannelUpdate::NewState { 
                        best_tip: state.best_tip_block().clone(), 
                        ledger: state.best_ledger()?.unwrap(), 
                        summary: state.summary_verbose() 
                    }).await?;
                    ipc_update_sender.send(IpcChannelUpdate::NewStore { 
                        store: Arc::new(state.spawn_secondary_database()?) 
                    }).await?;
                } else {
                    info!("Block receiver shutdown, system exit");
                    return Ok(())
                }
            }
        }
    }
}

#[instrument(skip_all)]
async fn handle_conn(
    conn: LocalSocketStream,
    db: Arc<IndexerStore>,
    best_tip: &Block,
    ledger: Ledger,
    summary: Option<SummaryVerbose>,
) -> Result<(), anyhow::Error> {
    let (reader, mut writer) = conn.into_split();
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::with_capacity(1024);
    let read_size = reader.read_until(0, &mut buffer).await?;
    if read_size == 0 {
        return Err(anyhow!("Unexpected EOF"));
    }
    let mut buffers = buffer.split(|byte| *byte == b' ');
    let command = buffers.next().unwrap();
    let command_string = String::from_utf8(command.to_vec()).unwrap();

    let response_json = match command_string.as_str() {
        "account" => {
            let data_buffer = buffers.next().unwrap();
            let public_key = PublicKey::from_address(&String::from_utf8(
                data_buffer[..data_buffer.len() - 1].to_vec(),
            )?)?;
            info!("Received account command for {public_key:?}");
            trace!("Using ledger {ledger:?}");
            let account = ledger.accounts.get(&public_key);
            if let Some(account) = account {
                debug!("Writing account {account:?} to client");
                Some(serde_json::to_string(account)?)
            } else {
                None
            }
        }
        "best_chain" => {
            info!("Received best_chain command");
            let data_buffer = buffers.next().unwrap();
            let num = String::from_utf8(data_buffer[..data_buffer.len() - 1].to_vec())?
                .parse::<usize>()?;
            let mut parent_hash = best_tip.parent_hash.clone();
            let mut best_chain = vec![db.get_block(&best_tip.state_hash)?.unwrap()];
            for _ in 1..num {
                let parent_pcb = db.get_block(&parent_hash)?.unwrap();
                parent_hash =
                    BlockHash::from_hashv1(parent_pcb.protocol_state.previous_state_hash.clone());
                best_chain.push(parent_pcb);
            }
            Some(serde_json::to_string(&best_chain)?)
        }
        "best_ledger" => {
            info!("Received best_ledger command");
            let data_buffer = buffers.next().unwrap();
            let path = &String::from_utf8(data_buffer[..data_buffer.len() - 1].to_vec())?
                .parse::<PathBuf>()?;
            if !path.is_dir() {
                debug!("Writing ledger to {}", path.display());
                fs::write(path, format!("{ledger:?}")).await?;
                Some(serde_json::to_string(&format!(
                    "Ledger written to {}",
                    path.display()
                ))?)
            } else {
                Some(serde_json::to_string(&format!(
                    "The path provided must be a file: {}",
                    path.display()
                ))?)
            }
        }
        "summary" => {
            info!("Received summary command");
            let data_buffer = buffers.next().unwrap();
            let verbose = String::from_utf8(data_buffer[..data_buffer.len() - 1].to_vec())?
                .parse::<bool>()?;
            if let Some(summary) = summary {
                if verbose {
                    Some(serde_json::to_string::<SummaryVerbose>(&summary)?)
                } else {
                    Some(serde_json::to_string::<SummaryShort>(&summary.into())?)
                }
            } else {
                Some(serde_json::to_string(&String::from("No summary available yet"))?)
            }
        }
        bad_request => {
            return Err(anyhow!("Malformed request: {bad_request}"));
        }
    };

    if let Some(response_json) = response_json {
        writer.write_all(response_json.as_bytes()).await?;
    } else {
        writer
            .write_all(serde_json::to_string("no response 404")?.as_bytes())
            .await?;
    }

    Ok(())
}

pub async fn create_dir_if_non_existent(path: &str) {
    if metadata(path).await.is_err() {
        debug!("Creating directory {path}");
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