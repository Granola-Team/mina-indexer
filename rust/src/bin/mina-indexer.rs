use clap::{Parser, Subcommand};
use log::{error, info, trace, LevelFilter};
use mina_indexer::{
    block::precomputed::PcbVersion,
    chain::Network,
    client,
    constants::*,
    ledger::{
        self,
        genesis::{GenesisConstants, GenesisLedger},
    },
    server::{IndexerConfiguration, InitializationMode, MinaIndexer},
    store::IndexerStore,
};
use std::{fs, path::PathBuf, str::FromStr, sync::Arc};
use stderrlog::{ColorChoice, Timestamp};

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
    #[arg(long, value_name = "FILE")]
    genesis_ledger: Option<PathBuf>,

    /// Hash of the initial state
    #[arg(
        long,
        default_value = MAINNET_GENESIS_HASH
    )]
    genesis_hash: String,

    /// Path to the genesis constants (JSON)
    genesis_constants: Option<PathBuf>,

    /// Override the constraint system digests
    constraint_system_digests: Option<Vec<String>>,

    /// Directory containing the precomputed blocks
    #[arg(long)]
    blocks_dir: Option<PathBuf>,

    /// Directory to watch for new precomputed blocks
    #[arg(long)]
    block_watch_dir: Option<PathBuf>,

    /// Directory containing the staking ledgers
    #[arg(long)]
    staking_ledgers_dir: Option<PathBuf>,

    /// Directory to watch for new staking ledgers
    #[arg(long)]
    staking_ledger_watch_dir: Option<PathBuf>,

    /// Path to directory for speedb
    #[arg(long, default_value = "/var/log/mina-indexer/database")]
    pub database_dir: PathBuf,

    /// Max stdout log level
    #[arg(long, default_value_t = LevelFilter::Warn)]
    pub log_level: LevelFilter,

    /// Number of blocks to add to the canonical chain before persisting a
    /// ledger snapshot
    #[arg(long, default_value_t = LEDGER_CADENCE)]
    ledger_cadence: u32,

    /// Number of blocks to process before reporting progress
    #[arg(long, default_value_t = BLOCK_REPORTING_FREQ_NUM)]
    reporting_freq: u32,

    /// Interval for pruning the root branch
    #[arg(long, default_value_t = PRUNE_INTERVAL_DEFAULT)]
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

    /// Path to the missing block recovery executable
    #[arg(long)]
    missing_block_recovery_exe: Option<PathBuf>,

    /// Delay (sec) in between missing block recovery attempts
    #[arg(long)]
    missing_block_recovery_delay: Option<u64>,

    /// Recover all blocks at all missing heights
    #[arg(long)]
    missing_block_recovery_batch: Option<bool>,

    /// Network name
    #[arg(long, default_value = Network::Mainnet)]
    network: Network,

    /// Domain socket path
    domain_socket_path: Option<PathBuf>,

    /// Indexer process ID
    pid: Option<u32>,
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct ConfigArgs {
    /// Path to the server config file
    #[arg(short, long)]
    path: Option<PathBuf>,
}

impl ServerArgs {
    fn with_dynamic_defaults(mut self, domain_socket_path: PathBuf, pid: u32) -> Self {
        if self.locked_supply_csv.is_none() {
            let path = match release_profile() {
                ReleaseProfile::Production => PathBuf::from("/share/mina-indexer/data/locked.csv"),
                ReleaseProfile::Development => concat!(env!("PWD"), "/data/locked.csv").into(),
            };
            self.locked_supply_csv = Some(path);
        }
        if self.genesis_ledger.is_none() {
            let ledger_path = match release_profile() {
                ReleaseProfile::Production => {
                    PathBuf::from("/share/mina-indexer/data/mainnet.json")
                }
                ReleaseProfile::Development => {
                    concat!(env!("PWD"), "/tests/data/genesis_ledgers/mainnet.json").into()
                }
            };
            self.genesis_ledger = Some(ledger_path);
        }
        self.pid = Some(pid);
        self.domain_socket_path = Some(domain_socket_path);
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

pub const DEFAULT_BLOCKS_DIR: &str = "/share/mina-indexer/blocks";
pub const DEFAULT_STAKING_LEDGERS_DIR: &str = "/share/mina-indexer/staking-ledgers";

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
            let args = args.with_dynamic_defaults(domain_socket_path.clone(), std::process::id());
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
            stderrlog::new()
                .module(module_path!())
                .color(ColorChoice::Never)
                .timestamp(Timestamp::Microsecond)
                .verbosity(args.log_level)
                .init()
                .unwrap();

            // log server config
            let args_json: ServerArgsJson = args.clone().into();
            info!(
                "Indexer config:\n{}",
                serde_json::to_string_pretty(&args_json)?
            );

            trace!("Building an indexer configuration");
            let config = process_indexer_configuration(args, mode)?;

            trace!("Creating a new IndexerStore in {}", database_dir.display());
            let db = Arc::new(IndexerStore::new(&database_dir)?);

            trace!(
                "Creating an Indexer listening on {}",
                domain_socket_path.display()
            );
            let indexer = MinaIndexer::new(config, db.clone()).await?;

            trace!(
                "Starting the HTTP server listening on {}:{}",
                web_hostname,
                web_port
            );
            mina_indexer::web::start_web_server(
                db.clone(),
                (web_hostname, web_port),
                locked_supply_csv,
            )
            .await
            .unwrap();
            indexer.await_loop().await;

            info!("Cleanly shutting down primary rocksdb");
            drop(db);
            Ok(())
        }
    }
}

pub fn process_indexer_configuration(
    args: ServerArgs,
    mode: InitializationMode,
) -> anyhow::Result<IndexerConfiguration> {
    let ledger = args.genesis_ledger.expect("Genesis ledger wasn't provided");
    let genesis_hash = args.genesis_hash.into();
    let blocks_dir = args.blocks_dir;
    let block_watch_dir = args
        .block_watch_dir
        .unwrap_or(blocks_dir.clone().unwrap_or(DEFAULT_BLOCKS_DIR.into()));
    let staking_ledgers_dir = args.staking_ledgers_dir;
    let staking_ledger_watch_dir = args.staking_ledger_watch_dir.unwrap_or(
        staking_ledgers_dir
            .clone()
            .unwrap_or(DEFAULT_STAKING_LEDGERS_DIR.into()),
    );
    let prune_interval = args.prune_interval;
    let canonical_threshold = args.canonical_threshold;
    let canonical_update_threshold = args.canonical_update_threshold;
    let ledger_cadence = args.ledger_cadence;
    let reporting_freq = args.reporting_freq;
    let domain_socket_path = args
        .domain_socket_path
        .unwrap_or("./mina-indexer.sock".into());
    let missing_block_recovery_exe = args.missing_block_recovery_exe;
    let missing_block_recovery_delay = args.missing_block_recovery_delay;
    let missing_block_recovery_batch = args.missing_block_recovery_batch.unwrap_or(false);

    // pick up genesis constants from the given file or use defaults
    let genesis_constants = {
        let mut constants = GenesisConstants::default();
        if let Some(path) = args.genesis_constants {
            if let Ok(ref contents) = std::fs::read(path) {
                if let Ok(override_constants) = serde_json::from_slice::<GenesisConstants>(contents)
                {
                    constants.override_with(override_constants);
                } else {
                    error!(
                        "Error parsing supplied genesis constants. Using default constants:\n{}",
                        serde_json::to_string_pretty(&constants)?
                    )
                }
            } else {
                error!(
                    "Error reading genesis constants file. Using default constants:\n{}",
                    serde_json::to_string_pretty(&constants)?
                )
            }
        }
        constants
    };
    let constraint_system_digests = args.constraint_system_digests.unwrap_or(
        MAINNET_CONSTRAINT_SYSTEM_DIGESTS
            .iter()
            .map(|x| x.to_string())
            .collect(),
    );

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

    trace!(
        "Creating block watch directories if missing: {}",
        block_watch_dir.display()
    );
    fs::create_dir_all(block_watch_dir.clone())?;

    trace!(
        "Creating ledger watch directories if missing: {}",
        staking_ledger_watch_dir.display()
    );
    fs::create_dir_all(staking_ledger_watch_dir.clone())?;

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
                genesis_constants,
                constraint_system_digests,
                version: PcbVersion::V1, // TODO make configurable
                blocks_dir,
                block_watch_dir,
                staking_ledgers_dir,
                staking_ledger_watch_dir,
                prune_interval,
                canonical_threshold,
                canonical_update_threshold,
                initialization_mode: mode,
                ledger_cadence,
                reporting_freq,
                domain_socket_path,
                missing_block_recovery_exe,
                missing_block_recovery_delay,
                missing_block_recovery_batch,
            })
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ServerArgsJson {
    genesis_ledger: String,
    genesis_hash: String,
    genesis_constants: Option<String>,
    constraint_system_digests: Option<Vec<String>>,
    blocks_dir: Option<String>,
    block_watch_dir: String,
    staking_ledgers_dir: Option<String>,
    staking_ledger_watch_dir: String,
    database_dir: String,
    log_level: String,
    ledger_cadence: u32,
    reporting_freq: u32,
    prune_interval: u32,
    canonical_threshold: u32,
    canonical_update_threshold: u32,
    locked_supply_csv: Option<String>,
    web_hostname: String,
    web_port: u16,
    pid: Option<u32>,
    domain_socket_path: Option<String>,
    missing_block_recovery_exe: Option<String>,
    missing_block_recovery_delay: Option<u64>,
    missing_block_recovery_batch: Option<bool>,
    network: String,
}

impl From<ServerArgs> for ServerArgsJson {
    fn from(value: ServerArgs) -> Self {
        let pid = value.pid.unwrap();
        let domain_socket_path = value.domain_socket_path.clone().unwrap();
        let value = value.with_dynamic_defaults(domain_socket_path, pid);
        Self {
            genesis_ledger: value
                .genesis_ledger
                .expect("Genesis ledger wasn't provided")
                .display()
                .to_string(),
            genesis_hash: value.genesis_hash,
            genesis_constants: value.genesis_constants.map(|g| g.display().to_string()),
            constraint_system_digests: value.constraint_system_digests,
            blocks_dir: value.blocks_dir.map(|d| d.display().to_string()),
            block_watch_dir: value
                .block_watch_dir
                .unwrap_or(DEFAULT_BLOCKS_DIR.into())
                .display()
                .to_string(),
            staking_ledgers_dir: value.staking_ledgers_dir.map(|d| d.display().to_string()),
            staking_ledger_watch_dir: value
                .staking_ledger_watch_dir
                .unwrap_or(DEFAULT_STAKING_LEDGERS_DIR.into())
                .display()
                .to_string(),
            database_dir: value.database_dir.display().to_string(),
            log_level: value.log_level.to_string(),
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
            pid: value.pid,
            domain_socket_path: value.domain_socket_path.map(|s| s.display().to_string()),
            missing_block_recovery_delay: value.missing_block_recovery_delay,
            missing_block_recovery_exe: value
                .missing_block_recovery_exe
                .map(|p| p.display().to_string()),
            missing_block_recovery_batch: value.missing_block_recovery_batch,
            network: format!("{}", value.network),
        }
    }
}

impl From<ServerArgsJson> for ServerArgs {
    fn from(value: ServerArgsJson) -> Self {
        Self {
            genesis_ledger: value.genesis_ledger.parse().ok(),
            genesis_hash: value.genesis_hash,
            genesis_constants: value.genesis_constants.map(|g| g.into()),
            constraint_system_digests: value.constraint_system_digests,
            blocks_dir: value.blocks_dir.map(|d| d.into()),
            block_watch_dir: Some(value.block_watch_dir.into()),
            staking_ledgers_dir: value.staking_ledgers_dir.map(|d| d.into()),
            staking_ledger_watch_dir: Some(value.staking_ledger_watch_dir.into()),
            database_dir: value.database_dir.into(),
            log_level: LevelFilter::from_str(&value.log_level).expect("log level"),
            ledger_cadence: value.ledger_cadence,
            reporting_freq: value.reporting_freq,
            prune_interval: value.prune_interval,
            canonical_threshold: value.canonical_threshold,
            canonical_update_threshold: value.canonical_update_threshold,
            locked_supply_csv: value.locked_supply_csv.map(|p| p.into()),
            web_hostname: value.web_hostname,
            web_port: value.web_port,
            pid: value.pid,
            domain_socket_path: value.domain_socket_path.map(|s| s.into()),
            missing_block_recovery_delay: value.missing_block_recovery_delay,
            missing_block_recovery_exe: value.missing_block_recovery_exe.map(|p| p.into()),
            missing_block_recovery_batch: value.missing_block_recovery_batch,
            network: (&value.network as &str).into(),
        }
    }
}
