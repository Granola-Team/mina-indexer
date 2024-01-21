use clap::{Parser, Subcommand};
use mina_indexer::{
    client,
    constants::{
        CANONICAL_UPDATE_THRESHOLD, MAINNET_CANONICAL_THRESHOLD, MAINNET_GENESIS_HASH,
        MAINNET_TRANSITION_FRONTIER_K, PRUNE_INTERVAL_DEFAULT,
    },
    ledger,
    server::{IndexerConfiguration, InitializationMode, MinaIndexer},
    store::IndexerStore,
};
use std::{fs, path::PathBuf, sync::Arc};
use tracing::{error, info, instrument};
use tracing_subscriber::{filter::LevelFilter, prelude::*};

#[derive(Parser, Debug)]
#[command(name = "mina-indexer", author, version, about, long_about = Some("Mina Indexer\n\n\
Efficiently index and query the Mina blockchain"))]
struct Cli {
    #[command(subcommand)]
    command: IndexerCommand,
}

#[derive(Subcommand, Debug)]
enum IndexerCommand {
    /// Server commands
    Server {
        #[command(subcommand)]
        server_command: ServerCommand,
    },
    /// Client commands
    #[clap(flatten)]
    Client(#[command(subcommand)] client::ClientCli),
}

#[derive(Subcommand, Debug)]
enum ServerCommand {
    /// Start the mina indexer by passing in arguments manually on the command line
    Cli(ServerArgs),
    /// Replay the events from an existing db to start the indexer
    Replay(ServerArgs),
    /// Sync from events in an existing db to start the indexer
    Sync(ServerArgs),
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct ServerArgs {
    /// Path to the genesis ledger
    #[arg(
        short,
        long,
        default_value = concat!(env!("PWD"), "/tests/data/genesis_ledgers/mainnet.json")
    )]
    genesis_ledger: PathBuf,
    /// Hash of the initial state
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
    pub database_dir: PathBuf,
    /// Path to directory for logs
    #[arg(long, default_value = concat!(env!("HOME"), "/.mina-indexer/logs"))]
    pub log_dir: PathBuf,
    /// Max file log level
    #[arg(long, default_value_t = LevelFilter::DEBUG)]
    pub log_level: LevelFilter,
    /// Max stdout log level
    #[arg(long, default_value_t = LevelFilter::INFO)]
    pub log_level_stdout: LevelFilter,
    /// Interval for pruning the root branch
    #[arg(short, long, default_value_t = PRUNE_INTERVAL_DEFAULT)]
    prune_interval: u32,
    /// Threshold for determining the canonicity of a block
    #[arg(long, default_value_t = MAINNET_CANONICAL_THRESHOLD)]
    canonical_threshold: u32,
    /// Threshold for updating the canonical tip/ledger
    #[arg(long, default_value_t = CANONICAL_UPDATE_THRESHOLD)]
    canonical_update_threshold: u32,
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        IndexerCommand::Client(args) => client::run(&args).await,
        IndexerCommand::Server { server_command } => {
            let mut is_sync = false;
            let mut is_replay = false;
            let args = match server_command {
                ServerCommand::Cli(args) => args,
                ServerCommand::Sync(args) => {
                    is_sync = true;
                    args
                }
                ServerCommand::Replay(args) => {
                    is_replay = true;
                    args
                }
            };
            let database_dir = args.database_dir.clone();
            if let Ok(dir) = std::fs::read_dir(database_dir.clone()) {
                if dir.count() != 0 {
                    // sync from existing db
                    is_sync = true;
                }
            }

            let log_dir = args.log_dir.clone();
            let log_level = args.log_level;
            let log_level_stdout = args.log_level_stdout;

            init_tracing_logger(log_dir, log_level, log_level_stdout).await?;

            let mode = if !is_replay && !is_sync {
                InitializationMode::New
            } else if is_replay {
                InitializationMode::Replay
            } else {
                InitializationMode::Sync
            };
            let config = process_indexer_configuration(args, mode)?;
            let db = Arc::new(IndexerStore::new(&database_dir)?);
            let indexer = MinaIndexer::new(config, db.clone()).await?;

            indexer.await_loop().await;
            Ok(())
        }
    }
}

async fn init_tracing_logger(
    log_dir: PathBuf,
    log_level: LevelFilter,
    log_level_stdout: LevelFilter,
) -> anyhow::Result<()> {
    let mut log_number = 0;
    let mut log_file = format!("{}/mina-indexer-{}.log", log_dir.display(), log_number);
    fs::create_dir_all(log_dir.clone()).expect("log_dir should be created");

    while tokio::fs::metadata(&log_file).await.is_ok() {
        log_number += 1;
        log_file = format!("{}/mina-indexer-{}.log", log_dir.display(), log_number);
    }
    let log_file = PathBuf::from(log_file);

    // setup tracing
    if let Some(parent) = log_file.parent() {
        fs::create_dir_all(parent).expect("log_file parent should be created");
    }

    let log_file = std::fs::File::create(log_file)?;
    let file_layer = tracing_subscriber::fmt::layer().with_writer(log_file);

    let stdout_layer = tracing_subscriber::fmt::layer();
    tracing_subscriber::registry()
        .with(stdout_layer.with_filter(log_level_stdout))
        .with(file_layer.with_filter(log_level))
        .init();
    Ok(())
}

#[instrument(skip_all)]
pub fn process_indexer_configuration(
    args: ServerArgs,
    mode: InitializationMode,
) -> anyhow::Result<IndexerConfiguration> {
    let ledger = args.genesis_ledger;
    let root_hash = args.root_hash.into();
    let startup_dir = args.startup_dir;
    let watch_dir = args.watch_dir;
    let prune_interval = args.prune_interval;
    let canonical_threshold = args.canonical_threshold;
    let canonical_update_threshold = args.canonical_update_threshold;

    assert!(
        ledger.is_file(),
        "Ledger file does not exist at {}",
        ledger.display()
    );
    assert!(
        // bad things happen if this condition fails
        canonical_update_threshold < MAINNET_TRANSITION_FRONTIER_K,
        "canonical update threshold must be strictly less than the transition frontier length!"
    );
    fs::create_dir_all(watch_dir.clone()).expect("watch_dir should be created");

    info!("Parsing ledger file at {}", ledger.display());

    match ledger::genesis::parse_file(&ledger) {
        Err(err) => {
            error!("Unable to parse genesis ledger: {err}");
            std::process::exit(100)
        }
        Ok(ledger) => {
            info!("Ledger parsed successfully!");

            Ok(IndexerConfiguration {
                ledger,
                root_hash,
                startup_dir,
                watch_dir,
                prune_interval,
                canonical_threshold,
                canonical_update_threshold,
                initialization_mode: mode,
            })
        }
    }
}
