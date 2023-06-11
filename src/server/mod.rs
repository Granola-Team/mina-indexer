use crate::{
    block::{
        parser::BlockParser, precomputed::PrecomputedBlock, receiver::BlockReceiver,
        store::BlockStoreConn, BlockHash,
    },
    state::{
        ledger::{self, genesis::GenesisRoot, public_key::PublicKey, Ledger},
        summary::{DbStats, Summary},
        IndexerState,
    },
    SOCKET_NAME, MAINNET_TRANSITION_FRONTIER_K,
};
use clap::{Args, Parser};
use futures::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use interprocess::local_socket::tokio::{LocalSocketListener, LocalSocketStream};
use std::{path::PathBuf, process, str::FromStr};
use tokio::time::Instant;
use tracing::{debug, error, info, instrument};
use uuid::Uuid;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct ServerArgs {
    /// Path to genesis ledger
    #[arg(short, long)]
    genesis_ledger: PathBuf,
    /// Hash of startup ledger
    #[arg(short, long)]
    root_hash: Option<String>,
    /// Path to startup blocks directory
    #[arg(short, long)]
    startup_dir: Option<PathBuf>,
    /// Path to directory to watch for new blocks
    #[arg(short, long)]
    watch_dir: Option<PathBuf>,
    /// Path to directory for rocksdb
    #[arg(short, long)]
    database_dir: Option<PathBuf>,
    /// Path to directory for logs (default: stdout)
    #[arg(short, long)]
    log_dir: Option<PathBuf>,
    /// Override an existing db
    #[arg(short, long, default_value_t = false)]
    db_override: bool,
}

#[derive(Args, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct GenesisPath {
    genesis_ledger: String,
}

pub struct IndexerConfiguration {
    genesis_ledger: GenesisRoot,
    root_hash: BlockHash,
    startup_dir: PathBuf,
    watch_dir: PathBuf,
    database_dir: PathBuf,
    log_file: Option<PathBuf>,
}

#[instrument(level = "info")]
pub async fn handle_command_line_arguments(
    args: ServerArgs,
) -> anyhow::Result<IndexerConfiguration> {
    debug!("Parsing server args");
    let root_hash = BlockHash(args.root_hash.unwrap_or({
        info!("Using default root hash: 3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ");
        "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ".to_string()
    }));
    let startup_dir = args.startup_dir.unwrap();
    let watch_dir = args.watch_dir.unwrap();
    let database_dir = args.database_dir.unwrap();
    let log_dir = args.log_dir;

    info!(
        "Parsing genesis ledger file at {}",
        args.genesis_ledger.display()
    );
    match ledger::genesis::parse_file(&args.genesis_ledger).await {
        Err(err) => {
            error!(
                reason = "Unable to parse genesis ledger",
                error = err.to_string(),
                path = &args.genesis_ledger.display().to_string()
            );
            process::exit(100)
        }
        Ok(genesis_ledger) => {
            info!("Genesis ledger parsed successfully!");

            let log_file;
            if let Some(log_dir) = log_dir {
                let mut log_number = 0;
                let mut log_fname =
                    format!("{}/mina-indexer-log-{}", log_dir.display(), log_number);
                while tokio::fs::metadata(&log_fname).await.is_ok() {
                    log_number += 1;
                    log_fname = format!("{}/mina-indexer-log-{}", log_dir.display(), log_number);
                }
                log_file = Some(PathBuf::from(&log_fname));
            } else {
                log_file = None;
            }
            Ok(IndexerConfiguration {
                genesis_ledger,
                root_hash,
                startup_dir,
                watch_dir,
                database_dir,
                log_file,
            })
        }
    }
}

#[instrument(level = "info")]
pub async fn run(args: ServerArgs) -> Result<(), anyhow::Error> {
    debug!("Checking that a server instance isn't already running");
    LocalSocketStream::connect(SOCKET_NAME)
        .await
        .expect_err("Server is already running... Exiting.");

    info!("Starting mina-indexer server");
    let IndexerConfiguration {
        genesis_ledger,
        root_hash,
        startup_dir,
        watch_dir,
        database_dir,
        log_file,
    } = handle_command_line_arguments(args).await?;

    let (non_blocking, _guard) = match log_file {
        None => tracing_appender::non_blocking(std::io::stdout()),
        Some(log_file) => {
            let log_writer = std::fs::File::create(log_file)?;
            tracing_appender::non_blocking(log_writer)
        }
    };
    tracing_subscriber::fmt().with_writer(non_blocking).init();

    info!("Initializing indexer state");
    let mut indexer_state = IndexerState::new(
        root_hash.clone(),
        genesis_ledger.ledger,
        Some(&database_dir),
        Some(MAINNET_TRANSITION_FRONTIER_K),
        Some(100),
    )?;

    // TODO check if db has values and reconstitute state from db block first
    let init_dir = startup_dir.display().to_string();
    info!("Ingesting precomputed blocks from {init_dir}");
    let mut block_parser = BlockParser::new(&startup_dir)?;
    let mut block_count = 0;
    let ingestion_time = Instant::now();
    while let Some(block) = block_parser.next().await? {
        debug!("Adding {:?} to the state", &block.state_hash);
        indexer_state.add_block(&block)?;
        block_count += 1;
    }
    info!(
        "Ingested {block_count} blocks from {init_dir} in {:?}",
        ingestion_time.elapsed()
    );

    let mut block_receiver = BlockReceiver::new().await?;
    info!("Block receiver set to watch {watch_dir:?}");
    block_receiver.load_directory(&watch_dir).await?;

    let listener = LocalSocketListener::bind(SOCKET_NAME)?;
    info!("Local socket listener started");

    loop {
        tokio::select! {
            block_fut = block_receiver.recv() => {
                if let Some(block_result) = block_fut {
                    let precomputed_block = block_result?;
                    debug!("Receiving block {:?}", precomputed_block);
                    indexer_state.add_block(&precomputed_block)?;
                    info!("Added block {:?}", &precomputed_block.state_hash);
                } else {
                    info!("Block receiver shutdown, system exit");
                    return Ok(())
                }
            }

            conn_fut = listener.accept() => {
                let conn = conn_fut?;
                info!("Receiving connection");
                let best_chain = indexer_state.root_branch.longest_chain();

                let primary_path = database_dir.clone();
                let mut secondary_path = primary_path.clone();
                secondary_path.push(Uuid::new_v4().to_string());

                debug!("Spawning secondary readonly RocksDB instance");
                let block_store_readonly = BlockStoreConn::new_read_only(&primary_path, &secondary_path)?;

                let mut max_dangling_height = 0;
                let mut max_dangling_length = 0;
                for dangling in &indexer_state.dangling_branches {
                    if dangling.height() > max_dangling_height {
                        max_dangling_height = dangling.height();
                    }
                    if dangling.len() > max_dangling_length {
                        max_dangling_length = dangling.len();
                    }
                }
                let db_stats_str = indexer_state
                    .block_store
                    .as_ref()
                    .map(|db|
                        db.database
                        .property_value(rocksdb::properties::DBSTATS)
                        .unwrap()
                        .unwrap()
                    );
                let summary = Summary {
                    block_count: indexer_state.block_count,
                    date_time: indexer_state.date_time,
                    uptime: indexer_state.time.clone().elapsed(),
                    root_height: indexer_state.root_branch.height(),
                    root_length: indexer_state.root_branch.len(),
                    num_dangling: indexer_state.dangling_branches.len(),
                    max_dangling_height,
                    max_dangling_length,
                    db_stats: db_stats_str.map(|s| DbStats::from_str(&s).unwrap()),
                };

                let mut leaves = indexer_state.root_branch.leaves.values().cloned();
                let leaf = leaves.find(|leaf| &leaf.block.state_hash == best_chain.first().unwrap_or(&root_hash)).unwrap();
                let ledger = leaf.get_ledger().clone();

                tokio::spawn(async move {
                    debug!("Handling connection");
                    if let Err(e) = handle_conn(conn, block_store_readonly, best_chain, ledger, summary).await {
                        error!("Error handling connection\n{e}");
                    }
                    debug!("Removing readonly instance at {}", secondary_path.display());
                    tokio::fs::remove_dir_all(&secondary_path).await.ok();
                });
            }
        }
    }
}

#[instrument(level = "info")]
async fn handle_conn(
    conn: LocalSocketStream,
    db: BlockStoreConn,
    best_chain: Vec<BlockHash>,
    ledger: Ledger,
    summary: Summary,
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
        "best_chain\0" => {
            info!("Received best_chain command");
            let best_chain: Vec<PrecomputedBlock> = best_chain[..best_chain.len() - 1]
                .iter()
                .cloned()
                .map(|state_hash| db.get_block(&state_hash.0).unwrap().unwrap())
                .collect();
            let bytes = bcs::to_bytes(&best_chain)?;
            writer.write_all(&bytes).await?;
        }
        "best_ledger" => {
            info!("Received best_ledger command");
            let data_buffer = buffers.next().unwrap();
            let path = &String::from_utf8(data_buffer[..data_buffer.len() - 1].to_vec())?
                .parse::<PathBuf>()?;
            debug!("Writing ledger to {}", path.display());
            let bytes = bcs::to_bytes(&ledger)?;
            writer.write_all(&bytes).await?;
        }
        "summary\0" => {
            info!("Received summary command");
            let bytes = bcs::to_bytes(&summary)?;
            writer.write_all(&bytes).await?;
        }
        bad_request => {
            error!("Malformed request: {bad_request}");
            return Err(anyhow::Error::msg(format!(
                "Malformed request: {bad_request}"
            )));
        }
    }

    Ok(())
}
