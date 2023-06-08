use std::path::PathBuf;

use clap::Parser;
use futures::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use interprocess::local_socket::tokio::{LocalSocketListener, LocalSocketStream};
use mina_indexer::{
    block::{
        parser::BlockParser, precomputed::PrecomputedBlock, receiver::BlockReceiver,
        store::BlockStoreConn, BlockHash,
    },
    state::{
        branch::Leaf,
        ledger::{self, genesis::GenesisRoot, public_key::PublicKey, Ledger},
    },
};
use tracing::{event, instrument, Level};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct ServerArgs {
    #[arg(short, long)]
    genesis_ledger: PathBuf,
    #[arg(short, long)]
    root_hash: String,
    #[arg(short, long)]
    startup_dir: PathBuf,
    #[arg(short, long)]
    watch_dir: PathBuf,
    #[arg(short, long)]
    store_dir: PathBuf,
    #[arg(short, long)]
    log_dir: PathBuf,
    #[arg(short, long, default_value_t = false)]
    log_stdout: bool,
}

pub struct IndexerConfiguration {
    root_hash: BlockHash,
    startup_dir: PathBuf,
    watch_dir: PathBuf,
    store_dir: PathBuf,
    log_file: PathBuf,
    genesis_ledger: GenesisRoot,
    log_stdout: bool,
}

#[instrument]
pub async fn parse_command_line_arguments() -> anyhow::Result<IndexerConfiguration> {
    event!(Level::INFO, "parsing ServerArgs");
    let args = ServerArgs::parse();
    let root_hash = BlockHash(args.root_hash);
    let startup_dir = args.startup_dir;
    let watch_dir = args.watch_dir;
    let store_dir = args.store_dir;
    let log_dir = args.log_dir;
    let log_stdout = args.log_stdout;
    event!(Level::INFO, "parsing GenesisLedger file");
    match ledger::genesis::parse_file(&args.genesis_ledger).await {
        Err(err) => {
            event!(
                Level::ERROR,
                reason = "unable to parse GenesisLedger",
                error = err.to_string(),
                path = &args.genesis_ledger.display().to_string()
            );
            Err(err)
        }
        Ok(genesis_ledger) => {
            event!(
                Level::INFO,
                "GenesisLedger parsed {}",
                args.genesis_ledger.display().to_string()
            );

            let mut log_number = 0;
            let mut log_file = format!("{}/mina-indexer-log-{}", log_dir.display(), log_number);
            while tokio::fs::metadata(&log_file).await.is_ok() {
                log_number += 1;
                log_file = format!("{}/mina-indexer-log-{}", log_dir.display(), log_number);
            }
            let log_file = PathBuf::from(&log_file);

            Ok(IndexerConfiguration {
                root_hash,
                startup_dir,
                watch_dir,
                store_dir,
                log_file,
                log_stdout,
                genesis_ledger,
            })
        }
    }
}

#[instrument]
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    event!(Level::INFO, "started mina-indexer-server");
    let IndexerConfiguration {
        root_hash,
        startup_dir,
        watch_dir,
        store_dir,
        log_file,
        log_stdout,
        genesis_ledger,
    } = parse_command_line_arguments().await?;

    // TODO: Improve Logging Subscribers
    if log_stdout {
        let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());
        tracing_subscriber::fmt().with_writer(non_blocking).init();
    } else {
        let log_writer = std::fs::File::create(log_file)?;
        let (non_blocking, _guard) = tracing_appender::non_blocking(log_writer);
        tracing_subscriber::fmt().with_writer(non_blocking).init();
    }

    event!(Level::INFO, "initializing IndexerState");
    let mut indexer_state =
        mina_indexer::state::IndexerState::new(root_hash, genesis_ledger.ledger, Some(&store_dir))?;

    event!(
        Level::INFO,
        "fast forwarding IndexerState using precomputed blocks in {}",
        startup_dir.display().to_string()
    );
    let mut block_parser = BlockParser::new(&startup_dir)?;
    while let Some(block) = block_parser.next().await? {
        event!(
            Level::INFO,
            "adding {:?} to IndexerState",
            &block.state_hash
        );
        indexer_state.add_block(&block)?;
    }
    event!(Level::INFO, "IndexerState up to date {:?}", indexer_state);

    event!(Level::INFO, "initializing BlockReceiver in {:?}", watch_dir);
    let mut block_receiver = BlockReceiver::new().await?;
    block_receiver.load_directory(&watch_dir).await?;

    event!(Level::INFO, "starting LocalSocketListener");
    let listener = LocalSocketListener::bind(mina_indexer::SOCKET_NAME)?;

    loop {
        tokio::select! {
            block_fut = block_receiver.recv() => {
                if let Some(block_result) = block_fut {
                    let precomputed_block = block_result?;
                    event!(Level::INFO, "receiving block {:?}", precomputed_block);
                    indexer_state.add_block(&precomputed_block)?;
                    event!(Level::INFO, "added block {:?}", &precomputed_block.state_hash);
                } else {
                    event!(Level::INFO, "BlockReceiver shutdown, system exit");
                    return Ok(())
                }
            }

            conn_fut = listener.accept() => {
                let conn = conn_fut?;
                event!(Level::INFO, "receiving connection");
                let best_chain = indexer_state.best_chain.clone();

                let primary_path = store_dir.clone();
                let mut secondary_path = primary_path.clone();
                secondary_path.push(Uuid::new_v4().to_string());

                event!(Level::INFO, "spawning secondary readonly RocksDB instance");
                let block_store_readonly = BlockStoreConn::new_read_only(&primary_path, &secondary_path)?;
                tokio::spawn(async move {
                    event!(Level::INFO, "handling connection");
                    if let Err(e) = handle_conn(conn, block_store_readonly, best_chain).await {
                        event!(Level::ERROR, "Error handling connection {}", e);
                    }
                    event!(Level::INFO, "removing readonly instance");
                    tokio::fs::remove_dir_all(&secondary_path).await.ok();
                });
            }
        }
    }
}

#[instrument]
async fn handle_conn(
    conn: LocalSocketStream,
    db: BlockStoreConn,
    best_chain: Vec<Leaf<Ledger>>,
) -> Result<(), anyhow::Error> {
    let (reader, mut writer) = conn.into_split();
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::with_capacity(128);
    let _read = reader.read_until(0, &mut buffer).await?;

    let mut buffers = buffer.split(|byte| *byte == 32);
    let command = buffers.next().unwrap();

    let command_string = String::from_utf8(command.to_vec())?;
    dbg!(&command_string);
    match command_string.as_str() {
        "best_chain\0" => {
            event!(Level::INFO, "received best_chain command");
            dbg!(best_chain.clone());
            let best_chain: Vec<PrecomputedBlock> = best_chain[..best_chain.len() - 1]
                .iter()
                .cloned()
                .map(|leaf| leaf.block.state_hash)
                .map(|state_hash| db.get_block(&state_hash.0).unwrap().unwrap())
                .collect();
            let bytes = bcs::to_bytes(&best_chain)?;
            writer.write_all(&bytes).await?;
        }
        "account_balance\0" => {
            event!(Level::INFO, "received account_balance command");
            let data_buffer = buffers.next().unwrap();
            let public_key = PublicKey::from_address(&String::from_utf8(
                data_buffer[..data_buffer.len() - 1].to_vec(),
            )?)?;
            if let Some(block) = best_chain.first() {
                event!(Level::INFO, "using ledger {:?}", block.get_ledger());
                let account = block.get_ledger().accounts.get(&public_key);
                if let Some(account) = account {
                    event!(Level::INFO, "writing account {:?} to client", account);
                    let bytes = bcs::to_bytes(account)?;
                    writer.write_all(&bytes).await?;
                }
            }
        }
        bad_request => {
            event!(Level::ERROR, "malformed request: {}", bad_request);
            return Err(anyhow::Error::msg("Malformed Request"));
        }
    }

    Ok(())
}
