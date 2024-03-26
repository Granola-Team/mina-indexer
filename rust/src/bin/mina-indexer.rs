use clap::{Parser, Subcommand};
use mina_indexer::{
    client,
    constants::*,
    ledger::{self, genesis::GenesisLedger},
    server::{IndexerConfiguration, InitializationMode, MinaIndexer},
    store::IndexerStore,
};
use std::{fs, path::PathBuf, str::FromStr, sync::Arc};
use tracing::{error, info, instrument};
use tracing_subscriber::{filter::LevelFilter, prelude::*};

#[derive(Parser, Debug)]
#[command(name = "mina-indexer", author, version, about, long_about = Some("Mina Indexer\n\n\
Efficiently index and query the Mina blockchain"))]
struct Cli {
    #[command(subcommand)]
    command: IndexerCommand,
    /// Path to the Unix domain socket file
    #[arg(long, default_value = concat!(env!("PWD"), "/mina-indexer.sock"))]
    domain_socket_path: PathBuf,
}

#[derive(Subcommand, Debug)]
enum IndexerCommand {
    /// Server commands
    Server {
        #[command(subcommand)]
        server_command: Box<ServerCommand>,
    },
    /// Client commands
    #[clap(flatten)]
    Client(#[command(subcommand)] client::ClientCli),
}

#[derive(Subcommand, Debug)]
enum ServerCommand {
    /// Start a new mina indexer by passing arguments on the command line
    Start(ServerArgs),
    /// Start a new mina indexer via a config file
    StartViaConfig(ConfigArgs),
    /// Start a mina indexer by replaying events from an existing indexer store
    Replay(ServerArgs),
    /// Start a mina indexer by syncing from events in an existing indexer store
    Sync(ServerArgs),
    /// Shutdown the server
    Shutdown,
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct ServerArgs {
    /// Path to the genesis ledger (JSON)
    #[arg(short, long, value_name = "FILE")]
    genesis_ledger: Option<PathBuf>,

    /// Hash of the initial state
    #[arg(
        long,
        default_value = MAINNET_GENESIS_HASH
    )]
    genesis_hash: String,

    /// Directory containing the precomputed blocks
    #[arg(long, default_value = "/usr/share/mina-indexer/blocks")]
    blocks_dir: PathBuf,

    /// Directory to watch for new precomputed blocks [default: blocks_dir]
    #[arg(long)]
    block_watch_dir: Option<PathBuf>,

    /// Directory containing the staking ledgers
    #[arg(long, default_value = "/usr/share/mina-indexer/staking-ledgers")]
    ledgers_dir: PathBuf,

    /// Directory to watch for new staking ledgers [default: ledgers_dir]
    #[arg(long)]
    ledger_watch_dir: Option<PathBuf>,

    /// Path to directory for speedb
    #[arg(short, long, default_value = "/var/log/mina-indexer/database")]
    pub database_dir: PathBuf,

    /// Path to directory for logs
    #[arg(long, default_value = "/var/log/mina-indexer")]
    pub log_dir: PathBuf,

    /// Max stdout log level
    #[arg(long, default_value_t = LevelFilter::INFO)]
    pub log_level: LevelFilter,

    /// Max file log level
    #[arg(long, default_value_t = LevelFilter::DEBUG)]
    pub log_level_file: LevelFilter,

    /// Number of blocks to add to the canonical chain before persisting a
    /// ledger snapshot
    #[arg(long, default_value_t = LEDGER_CADENCE)]
    ledger_cadence: u32,

    /// Number of blocks to process before reporting progress
    #[arg(long, default_value_t = BLOCK_REPORTING_FREQ_NUM)]
    reporting_freq: u32,

    /// Interval for pruning the root branch
    #[arg(short, long, default_value_t = PRUNE_INTERVAL_DEFAULT)]
    prune_interval: u32,

    /// Threshold for determining the canonicity of a block
    #[arg(long, default_value_t = MAINNET_CANONICAL_THRESHOLD)]
    canonical_threshold: u32,

    /// Threshold for updating the canonical root/ledger
    #[arg(long, default_value_t = CANONICAL_UPDATE_THRESHOLD)]
    canonical_update_threshold: u32,

    /// Web server hostname for REST and GraphQL
    #[arg(long, default_value = "localhost")]
    web_hostname: String,

    /// Web server port for REST and GraphQL
    #[arg(long, default_value_t = 8080)]
    web_port: u16,

    /// Path to the locked supply file (CSV)
    #[arg(long, value_name = "FILE")]
    locked_supply_csv: Option<PathBuf>,
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct ConfigArgs {
    /// Path to the server config file
    #[arg(short, long)]
    path: Option<PathBuf>,
}

impl ServerArgs {
    fn with_dynamic_defaults(mut self) -> Self {
        if self.locked_supply_csv.is_none() {
            let path = match release_profile() {
                ReleaseProfile::Production => {
                    PathBuf::from("/usr/share/mina-indexer/data/locked.csv")
                }
                ReleaseProfile::Development => concat!(env!("PWD"), "/data/locked.csv").into(),
            };
            self.locked_supply_csv = Some(path);
        }
        if self.genesis_ledger.is_none() {
            let ledger_path = match release_profile() {
                ReleaseProfile::Production => {
                    PathBuf::from("/usr/share/mina-indexer/data/mainnet.json")
                }
                ReleaseProfile::Development => {
                    concat!(env!("PWD"), "/tests/data/genesis_ledgers/mainnet.json").into()
                }
            };
            self.genesis_ledger = Some(ledger_path);
        }
        self
    }
}

pub enum ReleaseProfile {
    Production,
    Development,
}

fn release_profile() -> ReleaseProfile {
    match std::env::var("RELEASE").unwrap_or_default().as_str() {
        "production" => ReleaseProfile::Production,
        _ => ReleaseProfile::Development,
    }
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let domain_socket_path = cli.domain_socket_path;

    match cli.command {
        IndexerCommand::Client(args) => client::run(&args, &domain_socket_path).await,
        IndexerCommand::Server { server_command } => {
            let (args, mut mode) = match *server_command {
                ServerCommand::Shutdown => {
                    return client::run(&client::ClientCli::Shutdown, &domain_socket_path).await;
                }
                ServerCommand::Start(args) => (args, InitializationMode::New),
                ServerCommand::Sync(args) => (args, InitializationMode::Sync),
                ServerCommand::Replay(args) => (args, InitializationMode::Replay),
                ServerCommand::StartViaConfig(args) => {
                    let contents = std::fs::read(args.path.expect("server args config file"))?;
                    let args: ServerArgsJson = serde_json::from_slice(&contents)?;
                    (args.into(), InitializationMode::New)
                }
            };
            let args = args.with_dynamic_defaults();
            let locked_supply_csv = args.locked_supply_csv.clone();
            let database_dir = args.database_dir.clone();
            let web_hostname = args.web_hostname.clone();
            let web_port = args.web_port;

            // default to sync if there's a nonempty db dir
            if let Ok(dir) = std::fs::read_dir(database_dir.clone()) {
                if matches!(mode, InitializationMode::New) && dir.count() != 0 {
                    // sync from existing db
                    mode = InitializationMode::Sync;
                }
            }

            // initialize logging
            let log_dir = args.log_dir.clone();
            let log_level_file = args.log_level_file;
            let log_level = args.log_level;
            init_tracing_logger(log_dir.clone(), log_level_file, log_level).await?;

            // log server config
            let args_json: ServerArgsJson = args.clone().into();
            info!(
                "Indexer config:\n{}",
                serde_json::to_string_pretty(&args_json)?
            );

            // start the servers
            let config = process_indexer_configuration(args, mode)?;
            let db = Arc::new(IndexerStore::new(&database_dir)?);
            let indexer = MinaIndexer::new(config, db.clone(), domain_socket_path).await?;
            mina_indexer::web::start_web_server(db, (web_hostname, web_port), locked_supply_csv)
                .await
                .unwrap();
            indexer.await_loop().await;
            Ok(())
        }
    }
}

async fn init_tracing_logger(
    log_dir: PathBuf,
    log_level_file: LevelFilter,
    log_level_stdout: LevelFilter,
) -> anyhow::Result<()> {
    let mut log_number = 0;
    let mut log_file = format!("{}/mina-indexer-{}.log", log_dir.display(), log_number);
    fs::create_dir_all(log_dir.clone()).expect("log_dir should be created");

    while tokio::fs::metadata(&log_file).await.is_ok() {
        log_number += 1;
        log_file = format!("{}/mina-indexer-{}.log", log_dir.display(), log_number);
    }

    // setup tracing
    let log_file = PathBuf::from(log_file);
    if let Some(parent) = log_file.parent() {
        fs::create_dir_all(parent).expect("log_file parent should be created");
    }

    let log_file = std::fs::File::create(log_file)?;
    let file_layer = tracing_subscriber::fmt::layer().with_writer(log_file);
    let stdout_layer = tracing_subscriber::fmt::layer();
    tracing_subscriber::registry()
        .with(stdout_layer.with_filter(log_level_stdout))
        .with(file_layer.with_filter(log_level_file))
        .init();
    Ok(())
}

#[instrument(skip_all)]
pub fn process_indexer_configuration(
    args: ServerArgs,
    mode: InitializationMode,
) -> anyhow::Result<IndexerConfiguration> {
    let ledger = args.genesis_ledger.expect("Genesis ledger wasn't provided");
    let genesis_hash = args.genesis_hash.into();
    let blocks_dir = args.blocks_dir;
    let block_watch_dir = args.block_watch_dir.unwrap_or(blocks_dir.clone());
    let ledgers_dir = args.ledgers_dir;
    let ledger_watch_dir = args.ledger_watch_dir.unwrap_or(ledgers_dir.clone());
    let prune_interval = args.prune_interval;
    let canonical_threshold = args.canonical_threshold;
    let canonical_update_threshold = args.canonical_update_threshold;
    let ledger_cadence = args.ledger_cadence;
    let reporting_freq = args.reporting_freq;
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
    fs::create_dir_all(block_watch_dir.clone())?;
    fs::create_dir_all(ledger_watch_dir.clone())?;

    info!("Parsing ledger file at {}", ledger.display());
    match ledger::genesis::parse_file(&ledger) {
        Err(err) => {
            error!("Unable to parse genesis ledger: {}", err);
            std::process::exit(100)
        }
        Ok(genesis_root) => {
            let genesis_ledger: GenesisLedger = genesis_root.into();
            info!("Genesis ledger parsed successfully");

            Ok(IndexerConfiguration {
                genesis_ledger,
                genesis_hash,
                blocks_dir,
                block_watch_dir,
                ledgers_dir,
                ledger_watch_dir,
                prune_interval,
                canonical_threshold,
                canonical_update_threshold,
                initialization_mode: mode,
                ledger_cadence,
                reporting_freq,
            })
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ServerArgsJson {
    genesis_ledger: String,
    genesis_hash: String,
    blocks_dir: String,
    block_watch_dir: String,
    ledgers_dir: String,
    ledger_watch_dir: String,
    database_dir: String,
    log_dir: String,
    log_level: String,
    log_level_file: String,
    ledger_cadence: u32,
    reporting_freq: u32,
    prune_interval: u32,
    canonical_threshold: u32,
    canonical_update_threshold: u32,
    locked_supply_csv: Option<String>,
    web_hostname: String,
    web_port: u16,
}

impl From<ServerArgs> for ServerArgsJson {
    fn from(value: ServerArgs) -> Self {
        let value = value.with_dynamic_defaults();
        Self {
            genesis_ledger: value
                .genesis_ledger
                .expect("Genesis ledger wasn't provided")
                .display()
                .to_string(),
            genesis_hash: value.genesis_hash,
            blocks_dir: value.blocks_dir.display().to_string(),
            block_watch_dir: value
                .block_watch_dir
                .unwrap_or(value.blocks_dir)
                .display()
                .to_string(),
            ledgers_dir: value.ledgers_dir.display().to_string(),
            ledger_watch_dir: value
                .ledger_watch_dir
                .unwrap_or(value.ledgers_dir)
                .display()
                .to_string(),
            database_dir: value.database_dir.display().to_string(),
            log_dir: value.log_dir.display().to_string(),
            log_level: value.log_level.to_string(),
            log_level_file: value.log_level_file.to_string(),
            ledger_cadence: value.ledger_cadence,
            reporting_freq: value.reporting_freq,
            prune_interval: value.prune_interval,
            canonical_threshold: value.canonical_threshold,
            canonical_update_threshold: value.canonical_update_threshold,
            locked_supply_csv: value
                .locked_supply_csv
                .and_then(|p| p.to_str().map(|s| s.to_owned())),
            web_hostname: value.web_hostname,
            web_port: value.web_port,
        }
    }
}

impl From<ServerArgsJson> for ServerArgs {
    fn from(value: ServerArgsJson) -> Self {
        Self {
            genesis_ledger: value.genesis_ledger.parse().ok(),
            genesis_hash: value.genesis_hash,
            blocks_dir: value.blocks_dir.into(),
            block_watch_dir: Some(value.block_watch_dir.into()),
            ledgers_dir: value.ledgers_dir.into(),
            ledger_watch_dir: Some(value.ledger_watch_dir.into()),
            database_dir: value.database_dir.into(),
            log_dir: value.log_dir.into(),
            log_level: LevelFilter::from_str(&value.log_level).expect("log level"),
            log_level_file: LevelFilter::from_str(&value.log_level_file).expect("log level file"),
            ledger_cadence: value.ledger_cadence,
            reporting_freq: value.reporting_freq,
            prune_interval: value.prune_interval,
            canonical_threshold: value.canonical_threshold,
            canonical_update_threshold: value.canonical_update_threshold,
            locked_supply_csv: value.locked_supply_csv.map(|p| p.into()),
            web_hostname: value.web_hostname,
            web_port: value.web_port,
        }
    }
}
