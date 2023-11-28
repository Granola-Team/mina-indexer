use clap::{Parser, Subcommand};
use mina_indexer::{
    block::BlockHash,
    client,
    server::{IndexerConfiguration, MinaIndexer},
    state::ledger,
    store::IndexerStore,
    CANONICAL_UPDATE_THRESHOLD, MAINNET_CANONICAL_THRESHOLD, MAINNET_GENESIS_HASH,
    MAINNET_TRANSITION_FRONTIER_K, PRUNE_INTERVAL_DEFAULT,
};
use serde::Deserializer;
use serde_derive::Deserialize;
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
    Client {
        /// Output JSON data when possible
        #[arg(short, long, default_value_t = false)]
        output_json: bool,
        #[command(subcommand)]
        args: client::ClientCli,
    },
}

#[derive(Subcommand, Debug)]
enum ServerCommand {
    /// Start the mina indexer with a config file
    Config {
        #[arg(short, long)]
        path: PathBuf,
    },
    /// Start the mina indexer by passing in arguments manually on the command line
    Cli(ServerArgs),
}

#[derive(Parser, Debug, Clone, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct ServerArgs {
    /// Path to the root ledger (if non-genesis, set --non-genesis-ledger and --root-hash)
    #[arg(long)]
    initial_ledger: PathBuf,
    /// Use a non-genesis ledger
    #[arg(long, default_value_t = false)]
    is_genesis_ledger: bool,
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
    pub database_dir: PathBuf,
    /// Path to directory for logs
    #[arg(long, default_value = concat!(env!("HOME"), "/.mina-indexer/logs"))]
    pub log_dir: PathBuf,
    /// Max file log level
    #[serde(deserialize_with = "level_filter_deserializer")]
    #[arg(long, default_value_t = LevelFilter::DEBUG)]
    pub log_level: LevelFilter,
    /// Max stdout log level
    #[serde(deserialize_with = "level_filter_deserializer")]
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
        IndexerCommand::Client { output_json, args } => client::run(&args, output_json).await,
        IndexerCommand::Server { server_command } => {
            let args = match server_command {
                ServerCommand::Cli(args) => args,
                ServerCommand::Config { path } => {
                    let config_file = tokio::fs::read(path).await?;
                    serde_yaml::from_reader(&config_file[..])?
                }
            };
            let database_dir = args.database_dir.clone();
            let log_dir = args.log_dir.clone();
            let log_level = args.log_level;
            let log_level_stdout = args.log_level_stdout;

            init_tracing_logger(log_dir, log_level, log_level_stdout).await?;
            let config = process_indexer_configuration(args)?;
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
pub fn process_indexer_configuration(args: ServerArgs) -> anyhow::Result<IndexerConfiguration> {
    let ledger = args.initial_ledger;
    let is_genesis_ledger = args.is_genesis_ledger;
    let root_hash = BlockHash(args.root_hash.to_string());
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
                is_genesis_ledger,
                root_hash,
                startup_dir,
                watch_dir,
                prune_interval,
                canonical_threshold,
                canonical_update_threshold,
            })
        }
    }
}

pub fn level_filter_deserializer<'de, D>(deserializer: D) -> Result<LevelFilter, D::Error>
where
    D: Deserializer<'de>,
{
    struct YAMLStringVisitor;

    impl<'de> serde::de::Visitor<'de> for YAMLStringVisitor {
        type Value = LevelFilter;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string containing yaml data")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            // unfortunately we lose some typed information
            // from errors deserializing the json string
            let level_filter_str: &str = serde_yaml::from_str(v).map_err(E::custom)?;
            match level_filter_str {
                "info" => Ok(LevelFilter::INFO),
                "debug" => Ok(LevelFilter::DEBUG),
                "error" => Ok(LevelFilter::ERROR),
                "trace" => Ok(LevelFilter::TRACE),
                "warn" => Ok(LevelFilter::TRACE),
                "off" => Ok(LevelFilter::OFF),
                other => Err(E::custom(format!("{} is not a valid level filter", other))),
            }
        }
    }

    deserializer.deserialize_any(YAMLStringVisitor)
}
