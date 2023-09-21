use clap::{Parser, Subcommand};
use mina_indexer::{
    block::BlockHash,
    client,
    server::{create_dir_if_non_existent, IndexerConfiguration, MinaIndexer},
    state::ledger,
    store::IndexerStore,
    CANONICAL_UPDATE_THRESHOLD, MAINNET_CANONICAL_THRESHOLD, MAINNET_GENESIS_HASH,
    MAINNET_TRANSITION_FRONTIER_K, PRUNE_INTERVAL_DEFAULT,
};
use serde::Deserializer;
use serde_derive::Deserialize;
use std::{path::PathBuf, sync::Arc};
use tracing::{error, info, instrument, trace};
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
    #[arg(short, long)]
    initial_ledger: PathBuf,
    /// Use a non-genesis ledger
    #[arg(short, long, default_value_t = false)]
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
    /// Only store canonical blocks in the db
    #[arg(short, long, default_value_t = false)]
    keep_non_canonical_blocks: bool,
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
    #[arg(short, long, default_value_t = MAINNET_CANONICAL_THRESHOLD)]
    canonical_threshold: u32,
    /// Threshold for updating the canonical tip/ledger
    #[arg(short, long, default_value_t = CANONICAL_UPDATE_THRESHOLD)]
    canonical_update_threshold: u32,
    /// Path to an indexer snapshot
    #[arg(long)]
    pub snapshot_path: Option<PathBuf>,
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
            let option_snapshot_path = args.snapshot_path.clone();
            let database_dir = args.database_dir.clone();
            let log_dir = args.log_dir.clone();
            let log_level = args.log_level;
            let log_level_stdout = args.log_level_stdout;
            let config = handle_command_line_arguments(args).await?;

            let mut log_number = 0;
            let mut log_file = format!("{}/mina-indexer-{}.log", log_dir.display(), log_number);
            create_dir_if_non_existent(log_dir.to_str().unwrap()).await;
            while tokio::fs::metadata(&log_file).await.is_ok() {
                log_number += 1;
                log_file = format!("{}/mina-indexer-{}.log", log_dir.display(), log_number);
            }
            let log_file = PathBuf::from(log_file);

            // setup tracing
            if let Some(parent) = log_file.parent() {
                create_dir_if_non_existent(parent.to_str().unwrap()).await;
            }

            let log_file = std::fs::File::create(log_file.clone())?;
            let file_layer = tracing_subscriber::fmt::layer().with_writer(log_file);

            let stdout_layer = tracing_subscriber::fmt::layer();
            tracing_subscriber::registry()
                .with(stdout_layer.with_filter(log_level_stdout))
                .with(file_layer.with_filter(log_level))
                .init();

            let db = if let Some(snapshot_path) = option_snapshot_path {
                let indexer_store = IndexerStore::from_backup(&snapshot_path, &database_dir)?;
                Arc::new(indexer_store)
            } else {
                Arc::new(IndexerStore::new(&database_dir)?)
            };

            MinaIndexer::new(config, db.clone()).await?;
            mina_indexer::gql::start_gql(db).await.unwrap();
            Ok(())
        }
    }
}

#[instrument(skip_all)]
pub async fn handle_command_line_arguments(
    args: ServerArgs,
) -> anyhow::Result<IndexerConfiguration> {
    trace!("Parsing server args");

    let ledger = args.initial_ledger;
    let is_genesis_ledger = args.is_genesis_ledger;
    let root_hash = BlockHash(args.root_hash.to_string());
    let startup_dir = args.startup_dir;
    let watch_dir = args.watch_dir;
    let keep_noncanonical_blocks = args.keep_non_canonical_blocks;
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

    create_dir_if_non_existent(watch_dir.to_str().unwrap()).await;

    info!("Parsing ledger file at {}", ledger.display());

    match ledger::genesis::parse_file(&ledger).await {
        Err(err) => {
            error!(
                reason = "Unable to parse ledger",
                error = err.to_string(),
                path = &ledger.display().to_string()
            );
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
                keep_noncanonical_blocks,
                prune_interval,
                canonical_threshold,
                canonical_update_threshold,
                from_snapshot: args.snapshot_path.is_some(),
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
