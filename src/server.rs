use crate::{
    block::{
        parser::BlockParser, receiver::BlockReceiver, store::BlockStore, Block, BlockHash,
        BlockWithoutHeight,
    },
    state::{
        ledger::{self, genesis::GenesisRoot, public_key::PublicKey, Ledger},
        summary::{SummaryShort, SummaryVerbose},
        IndexerMode, IndexerState,
    },
    store::IndexerStore,
    CANONICAL_UPDATE_THRESHOLD, MAINNET_GENESIS_HASH, MAINNET_TRANSITION_FRONTIER_K,
    PRUNE_INTERVAL_DEFAULT, SOCKET_NAME,
};
use clap::Parser;
use futures::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use interprocess::local_socket::tokio::{LocalSocketListener, LocalSocketStream};
use log::trace;
use serde_derive::{Deserialize, Serialize};
use std::{path::PathBuf, process, sync::Arc};
use tokio::{
    fs::{self, create_dir_all, metadata},
    sync::mpsc,
};
use tracing::{debug, error, info, instrument, level_filters::LevelFilter};
use uuid::Uuid;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct ServerArgs {
    /// Path to the root ledger (if non-genesis, set --non-genesis-ledger and --root-hash)
    #[arg(short, long)]
    ledger: PathBuf,
    /// Use a non-genesis ledger
    #[arg(short, long, default_value_t = false)]
    non_genesis_ledger: bool,
    /// Hash of the base ledger
    #[arg(
        long,
        default_value = MAINNET_GENESIS_HASH
    )]
    root_hash: String,
    /// Path to startup blocks directory
    #[arg(short, long, default_value = concat!(env!("HOME"), "/.mina-indexer/startup-blocks"))]
    startup_dir: PathBuf,
    /// Path to directory to watch for new blocks
    #[arg(short, long, default_value = concat!(env!("HOME"), "/.mina-indexer/watch-blocks"))]
    watch_dir: PathBuf,
    /// Path to directory for rocksdb
    #[arg(short, long, default_value = concat!(env!("HOME"), "/.mina-indexer/database"))]
    database_dir: PathBuf,
    /// Path to directory for logs
    #[arg(long, default_value = concat!(env!("HOME"), "/.mina-indexer/logs"))]
    log_dir: PathBuf,
    /// Only store canonical blocks in the db
    #[arg(short, long, default_value_t = false)]
    keep_non_canonical_blocks: bool,
    /// Max file log level
    #[arg(long, default_value_t = LevelFilter::DEBUG)]
    log_level: LevelFilter,
    /// Max stdout log level
    #[arg(long, default_value_t = LevelFilter::INFO)]
    log_level_stdout: LevelFilter,
    /// Interval for pruning the root branch
    #[arg(short, long, default_value_t = PRUNE_INTERVAL_DEFAULT)]
    prune_interval: u32,
    /// Threshold for updating the canonical tip/ledger
    #[arg(short, long, default_value_t = CANONICAL_UPDATE_THRESHOLD)]
    canonical_update_threshold: u32,
    /// Path to an indexer snapshot
    #[arg(long)]
    pub snapshot_path: Option<PathBuf>,
}

pub struct IndexerConfiguration {
    ledger: GenesisRoot,
    non_genesis_ledger: bool,
    root_hash: BlockHash,
    startup_dir: PathBuf,
    watch_dir: PathBuf,
    pub database_dir: PathBuf,
    keep_noncanonical_blocks: bool,
    pub log_file: PathBuf,
    pub log_level: LevelFilter,
    pub log_level_stdout: LevelFilter,
    prune_interval: u32,
    canonical_update_threshold: u32,
    from_snapshot: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct SaveCommand(PathBuf);

#[derive(Debug, Serialize, Deserialize)]
struct SaveResponse(String);

#[instrument(skip_all)]
pub async fn handle_command_line_arguments(
    args: ServerArgs,
) -> anyhow::Result<IndexerConfiguration> {
    trace!("Parsing server args");

    let non_genesis_ledger = args.non_genesis_ledger;
    let root_hash = BlockHash(args.root_hash.to_string());
    let startup_dir = args.startup_dir;
    let watch_dir = args.watch_dir;
    let database_dir = args.database_dir;
    let keep_noncanonical_blocks = args.keep_non_canonical_blocks;
    let log_dir = args.log_dir;
    let log_level = args.log_level;
    let log_level_stdout = args.log_level_stdout;
    let prune_interval = args.prune_interval;
    let canonical_update_threshold = args.canonical_update_threshold;

    assert!(
        // bad things happen if this condition fails
        canonical_update_threshold < MAINNET_TRANSITION_FRONTIER_K,
        "canonical update threshold must be strictly less than the transition frontier length!"
    );

    create_dir_if_non_existent(watch_dir.to_str().unwrap()).await;
    create_dir_if_non_existent(log_dir.to_str().unwrap()).await;

    info!("Parsing genesis ledger file at {}", args.ledger.display());

    match ledger::genesis::parse_file(&args.ledger).await {
        Err(err) => {
            error!(
                reason = "Unable to parse genesis ledger",
                error = err.to_string(),
                path = &args.ledger.display().to_string()
            );
            process::exit(100)
        }
        Ok(ledger) => {
            info!("Genesis ledger parsed successfully!");

            let mut log_number = 0;
            let mut log_fname = format!("{}/mina-indexer-{}.log", log_dir.display(), log_number);

            while tokio::fs::metadata(&log_fname).await.is_ok() {
                log_number += 1;
                log_fname = format!("{}/mina-indexer-{}.log", log_dir.display(), log_number);
            }

            Ok(IndexerConfiguration {
                ledger,
                non_genesis_ledger,
                root_hash,
                startup_dir,
                watch_dir,
                database_dir,
                keep_noncanonical_blocks,
                log_file: PathBuf::from(&log_fname),
                log_level,
                log_level_stdout,
                prune_interval,
                canonical_update_threshold,
                from_snapshot: args.snapshot_path.is_some(),
            })
        }
    }
}

#[instrument(skip_all)]
pub async fn run(
    config: IndexerConfiguration,
    indexer_store: Arc<IndexerStore>,
) -> Result<(), anyhow::Error> {
    debug!("Checking that a server instance isn't already running");
    LocalSocketStream::connect(SOCKET_NAME)
        .await
        .expect_err("Server is already running... Exiting.");

    info!("Starting mina-indexer server");
    let IndexerConfiguration {
        ledger,
        non_genesis_ledger,
        root_hash,
        startup_dir,
        watch_dir,
        database_dir,
        keep_noncanonical_blocks,
        log_file,
        log_level,
        log_level_stdout,
        prune_interval,
        canonical_update_threshold,
        from_snapshot,
    } = config;

    let mode = if keep_noncanonical_blocks {
        IndexerMode::Full
    } else {
        IndexerMode::Light
    };
    let mut indexer_state = if !from_snapshot {
        info!(
            "Initializing indexer state from blocks in {}",
            startup_dir.display()
        );
        IndexerState::new(
            mode,
            root_hash.clone(),
            ledger.ledger,
            indexer_store,
            MAINNET_TRANSITION_FRONTIER_K,
            prune_interval,
            canonical_update_threshold,
        )?
    } else {
        info!("initializing indexer state from snapshot");
        IndexerState::from_state_snapshot(
            indexer_store,
            MAINNET_TRANSITION_FRONTIER_K,
            prune_interval,
            canonical_update_threshold,
        )?
    };
    let mut block_parser = BlockParser::new(&startup_dir)?;
    if !non_genesis_ledger {
        indexer_state
            .initialize_with_contiguous_canonical(&mut block_parser)
            .await?;
    } else {
        indexer_state
            .initialize_without_contiguous_canonical(&mut block_parser)
            .await?;
    }

    let mut block_receiver = BlockReceiver::new().await?;
    block_receiver.load_directory(&watch_dir).await?;
    info!("Block receiver set to watch {watch_dir:?}");

    let listener = LocalSocketListener::bind(SOCKET_NAME)?;
    info!("Local socket listener started");

    let (save_tx, mut save_rx) = tokio::sync::mpsc::channel(1);
    let (mut save_resp_tx, save_resp_rx) = spmc::channel();
    let save_tx = Arc::new(save_tx);
    let save_resp_rx = Arc::new(save_resp_rx);

    loop {
        tokio::select! {
            block_fut = block_receiver.recv() => {
                if let Some(block_result) = block_fut {
                    let precomputed_block = block_result?;
                    let block = BlockWithoutHeight::from_precomputed(&precomputed_block);
                    debug!("Receiving block {block:?}");

                    indexer_state.add_block(&precomputed_block)?;
                    info!("Added {block:?}");
                } else {
                    info!("Block receiver shutdown, system exit");
                    return Ok(())
                }
            }

            conn_fut = listener.accept() => {
                let conn = conn_fut?;
                info!("Receiving connection");
                let best_tip = indexer_state.best_tip_block().clone();

                let primary_path = database_dir.clone();
                let mut secondary_path = primary_path.clone();
                secondary_path.push(Uuid::new_v4().to_string());

                debug!("Spawning secondary readonly RocksDB instance");
                let block_store_readonly = IndexerStore::new_read_only(&primary_path, &secondary_path)?;
                let summary = indexer_state.summary_verbose();
                let ledger = indexer_state.best_ledger()?.unwrap();

                let save_tx = save_tx.clone();
                let save_resp_rx = save_resp_rx.clone();

                // handle the connection
                tokio::spawn(async move {
                    debug!("Handling connection");
                    if let Err(e) = handle_conn(conn, block_store_readonly, best_tip, ledger, summary, save_tx, save_resp_rx).await {
                        error!("Error handling connection: {e}");
                    }

                    debug!("Removing readonly instance at {}", secondary_path.display());
                    tokio::fs::remove_dir_all(&secondary_path).await.ok();
                });
            }

            save_rx_fut = save_rx.recv() => {
                if let Some(SaveCommand(snapshot_path)) = save_rx_fut {
                    trace!("saving snapshot in {}", &snapshot_path.display());
                    match indexer_state.save_snapshot(snapshot_path) {
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
    db: IndexerStore,
    best_tip: Block,
    ledger: Ledger,
    summary: SummaryVerbose,
    save_tx: Arc<mpsc::Sender<SaveCommand>>,
    save_resp_rx: Arc<spmc::Receiver<Option<SaveResponse>>>,
) -> Result<(), anyhow::Error> {
    let (reader, mut writer) = conn.into_split();
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::with_capacity(128);
    let _read = reader.read_until(0, &mut buffer).await?;

    let mut buffers = buffer.split(|byte| *byte == 32);
    let command = buffers.next().unwrap();
    let command_string = String::from_utf8(command.to_vec())?;

    match command_string.as_str() {
        "account" => {
            let data_buffer = buffers.next().unwrap();
            let public_key = PublicKey::from_address(&String::from_utf8(
                data_buffer[..data_buffer.len() - 1].to_vec(),
            )?)?;
            info!("Received account command for {public_key:?}");
            debug!("Using ledger {ledger:?}");
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
            debug!("Writing ledger to {}", path.display());
            fs::write(path, format!("{ledger:?}")).await?;
            let bytes = bcs::to_bytes(&format!("Ledger written to {}", path.display()))?;
            writer.write_all(&bytes).await?;
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
            loop {
                if let Some(resp) = save_resp_rx.try_recv()? {
                    // we want to block here
                    trace!("received SaveResponse {:?}", resp);
                    let bytes = bcs::to_bytes(&resp)?;
                    writer.write_all(&bytes).await?;
                    break;
                }
            }
        }
        bad_request => {
            let err_msg = format!("Malformed request: {bad_request}");
            error!("{err_msg}");
            return Err(anyhow::Error::msg(err_msg));
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
