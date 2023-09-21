use crate::{
    block::{parser::BlockParser, store::BlockStore, Block, BlockHash, BlockWithoutHeight},
    receiver::{filesystem::FilesystemReceiver, BlockReceiver},
    state::{
        ledger::{genesis::GenesisRoot, public_key::PublicKey, Ledger},
        summary::{SummaryShort, SummaryVerbose},
        IndexerMode, IndexerState, Tip,
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
    time::Duration,
};
use tokio::{
    fs::{self, create_dir_all, metadata},
    io,
    sync::{mpsc, watch},
    task::JoinHandle,
};
use tracing::{debug, error, info, instrument};

pub struct IndexerConfiguration {
    pub ledger: GenesisRoot,
    pub is_genesis_ledger: bool,
    pub root_hash: BlockHash,
    pub startup_dir: PathBuf,
    pub watch_dir: PathBuf,
    pub keep_noncanonical_blocks: bool,
    pub prune_interval: u32,
    pub canonical_threshold: u32,
    pub canonical_update_threshold: u32,
    pub from_snapshot: bool,
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
    StateInitializedFromSnapshot,
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

impl MinaIndexer {
    pub async fn new(
        config: IndexerConfiguration,
        store: Arc<IndexerStore>,
    ) -> anyhow::Result<Self> {
        let (phase_sender, phase_receiver) = watch::channel(MinaIndexerRunPhase::JustStarted);

        let (query_sender, query_receiver) = mpsc::channel(1);

        let _loop_join_handle = tokio::spawn(async move {
            let watch_dir = config.watch_dir.clone();
            let (state, phase_sender) = initialize(config, store, phase_sender).await?;
            run(watch_dir, state, phase_sender, query_receiver).await
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
    phase_sender: watch::Sender<MinaIndexerRunPhase>,
) -> anyhow::Result<(IndexerState, watch::Sender<MinaIndexerRunPhase>)> {
    use MinaIndexerRunPhase::*;
    debug!("Checking that a server instance isn't already running");
    phase_sender.send_replace(ConnectingToIPCSocket);
    LocalSocketStream::connect(SOCKET_NAME)
        .await
        .expect_err("Server is already running... Exiting.");

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
        keep_noncanonical_blocks,
        prune_interval,
        canonical_threshold,
        canonical_update_threshold,
        from_snapshot,
    } = config;
    let mode = if keep_noncanonical_blocks {
        IndexerMode::Full
    } else {
        IndexerMode::Light
    };
    let state = if !from_snapshot {
        info!(
            "Initializing indexer state from blocks in {}",
            startup_dir.display()
        );
        let mut state = IndexerState::new(
            mode,
            root_hash.clone(),
            ledger.ledger,
            store,
            MAINNET_TRANSITION_FRONTIER_K,
            prune_interval,
            canonical_update_threshold,
        )?;

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
        state
    } else {
        info!("initializing indexer state from snapshot");
        let state = IndexerState::from_state_snapshot(
            store,
            MAINNET_TRANSITION_FRONTIER_K,
            prune_interval,
            canonical_update_threshold,
        )?;

        phase_sender.send_replace(StateInitializedFromSnapshot);
        state
    };

    Ok((state, phase_sender))
}

#[instrument(skip_all)]
pub async fn run(
    block_watch_dir: impl AsRef<Path>,
    mut state: IndexerState,
    phase_sender: watch::Sender<MinaIndexerRunPhase>,
    mut query_receiver: mpsc::Receiver<(
        MinaIndexerQuery,
        oneshot::Sender<MinaIndexerQueryResponse>,
    )>,
) -> Result<(), anyhow::Error> {
    use MinaIndexerRunPhase::*;

    phase_sender.send_replace(StartingBlockReceiver);
    let mut filesystem_receiver = FilesystemReceiver::new(1024, 64).await?;
    filesystem_receiver.load_directory(block_watch_dir.as_ref())?;
    info!("Block receiver set to watch {:?}", block_watch_dir.as_ref());

    phase_sender.send_replace(StartingIPCSocketListener);
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

    phase_sender.send_replace(StartingMainServerLoop);
    let (save_tx, mut save_rx) = tokio::sync::mpsc::channel(1);
    let (mut save_resp_tx, save_resp_rx) = spmc::channel();
    let save_tx = Arc::new(save_tx);
    let save_resp_rx = Arc::new(save_resp_rx);
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
                } else {
                    info!("Block receiver shutdown, system exit");
                    return Ok(())
                }
            }

            conn_fut = listener.accept() => {
                match conn_fut {
                    Ok(stream) => {
                        info!("Accepted client connection");
                        phase_sender.send_replace(ReceivingIPCConnection);

                        let best_tip = state.best_tip_block().clone();
                        let block_store_readonly = Arc::new(state.spawn_secondary_database()?);
                        let summary = state.summary_verbose();
                        let ledger = state.best_ledger()?.unwrap();

                        let save_tx = save_tx.clone();
                        let save_resp_rx = save_resp_rx.clone();

                        // handle the connection
                        tokio::spawn(async move {
                            debug!("Handling client connection");
                            if let Err(e) = handle_conn(stream, block_store_readonly.clone(), best_tip, ledger, summary, save_tx, save_resp_rx).await {
                                error!("Error handling connection: {e}");
                            }
                            debug!("Removing readonly instance at {}", block_store_readonly.db_path.clone().display());
                            tokio::fs::remove_dir_all(&block_store_readonly.db_path).await.ok();
                        });
                    }
                    Err(e) => {
                        error!("Error accepting connection: {}", e);
                    }
                }

            }

            save_rx_fut = save_rx.recv() => {
                if let Some(SaveCommand(snapshot_path)) = save_rx_fut {
                    phase_sender.send_replace(SavingStateSnapshot);
                    trace!("saving snapshot in {}", &snapshot_path.display());
                    match state.save_snapshot(snapshot_path) {
                        Ok(_) => save_resp_tx.send(Some(SaveResponse("snapshot created".to_string())))?,
                        Err(e) => save_resp_tx.send(Some(SaveResponse(e.to_string())))?,
                    }
                }
            }
        }
    }
}

#[instrument(skip_all)]
async fn handle_conn(
    conn: LocalSocketStream,
    db: Arc<IndexerStore>,
    best_tip: Block,
    ledger: Ledger,
    summary: SummaryVerbose,
    save_tx: Arc<mpsc::Sender<SaveCommand>>,
    _save_resp_rx: Arc<spmc::Receiver<Option<SaveResponse>>>,
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

    match command_string.as_str() {
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
                let bytes = bcs::to_bytes(account)?;
                writer.write_all(&bytes).await?;
            }
        }
        "best_chain" => {
            info!("Received best_chain command");
            let data_buffer = buffers.next().unwrap();
            let num = String::from_utf8(data_buffer[..data_buffer.len() - 1].to_vec())?
                .parse::<usize>()?;
            let mut parent_hash = best_tip.parent_hash;
            let mut best_chain = vec![db.get_block(&best_tip.state_hash)?.unwrap()];
            for _ in 1..num {
                let parent_pcb = db.get_block(&parent_hash)?.unwrap();
                parent_hash =
                    BlockHash::from_hashv1(parent_pcb.protocol_state.previous_state_hash.clone());
                best_chain.push(parent_pcb);
            }
            let bytes = bcs::to_bytes(&best_chain)?;
            writer.write_all(&bytes).await?;
        }
        "best_ledger" => {
            info!("Received best_ledger command");
            let data_buffer = buffers.next().unwrap();
            let path = &String::from_utf8(data_buffer[..data_buffer.len() - 1].to_vec())?
                .parse::<PathBuf>()?;
            if !path.is_dir() {
                debug!("Writing ledger to {}", path.display());
                fs::write(path, format!("{ledger:?}")).await?;
                let bytes = bcs::to_bytes(&format!("Ledger written to {}", path.display()))?;
                writer.write_all(&bytes).await?;
            } else {
                let bytes = bcs::to_bytes(&format!(
                    "The path provided must be a file: {}",
                    path.display()
                ))?;
                writer.write_all(&bytes).await?;
            }
        }
        "summary" => {
            info!("Received summary command");
            let data_buffer = buffers.next().unwrap();
            let verbose = String::from_utf8(data_buffer[..data_buffer.len() - 1].to_vec())?
                .parse::<bool>()?;
            if verbose {
                let bytes = bcs::to_bytes(&summary)?;
                writer.write_all(&bytes).await?;
            } else {
                let summary: SummaryShort = summary.into();
                let bytes = bcs::to_bytes(&summary)?;
                writer.write_all(&bytes).await?;
            }
        }
        "save_state" => {
            info!("Received save_state command");
            let data_buffer = buffers.next().unwrap();
            let snapshot_path = PathBuf::from(String::from_utf8(
                data_buffer[..data_buffer.len() - 1].to_vec(),
            )?);
            trace!("sending SaveCommand to primary indexer thread");
            save_tx.send(SaveCommand(snapshot_path)).await?;
            trace!("awaiting SaveResponse from primary indexer thread");
            writer.write_all(b"saving snapshot...").await?;
        }
        bad_request => {
            return Err(anyhow!("Malformed request: {bad_request}"));
        }
    }

    Ok(())
}

pub async fn create_dir_if_non_existent(path: &str) {
    if metadata(path).await.is_err() {
        debug!("Creating directory {path}");
        create_dir_all(path).await.unwrap();
    }
}
