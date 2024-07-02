use clap::{Parser, Subcommand};
use log::{debug, error, info, trace, LevelFilter};
use mina_indexer::{
    block::precomputed::PcbVersion,
    chain::Network,
    client,
    constants::*,
    ledger::{
        self,
        genesis::{GenesisConstants, GenesisLedger, GenesisRoot},
    },
    server::{start_indexer, IndexerConfiguration, InitializationMode},
    store::{self, version::IndexerStoreVersion, IndexerStore},
    web::start_web_server,
};
use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use stderrlog::{ColorChoice, Timestamp};
use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle, Toplevel};

#[derive(Parser, Debug)]
#[command(name = "mina-indexer", author, version = VERSION, about, long_about = Some("Mina Indexer\n\n\
Efficiently index and query the Mina blockchain"))]
struct Cli {
    #[command(subcommand)]
    command: IndexerCommand,
    /// Path to the Unix domain socket file
    #[arg(long, default_value = "./mina-indexer.sock", num_args = 1)]
    socket: PathBuf,
}

#[derive(Subcommand, Debug)]
#[allow(clippy::large_enum_variant)]
enum IndexerCommand {
    /// Server commands
    Server {
        #[command(subcommand)]
        server_command: ServerCommand,
    },
    /// Client commands
    #[clap(flatten)]
    Client(#[command(subcommand)] client::ClientCli),
    /// Database version
    DbVersion,
    /// Restore a snapshot of the Indexer store
    RestoreSnapshot {
        /// Full file path to the compressed snapshot file to restore
        #[arg(long)]
        snapshot_file_path: PathBuf,

        /// Full file path to the location to restore to
        #[arg(long)]
        restore_dir: PathBuf,
    },
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

    /// Indexer process ID
    #[arg(last = true)]
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
    fn with_dynamic_defaults(mut self, pid: u32) -> Self {
        self.pid = Some(pid);
        self
    }
}

pub const DEFAULT_BLOCKS_DIR: &str = "/share/mina-indexer/blocks";
pub const DEFAULT_STAKING_LEDGERS_DIR: &str = "/share/mina-indexer/staking-ledgers";

#[tokio::main]
#[allow(clippy::unused_unit)]
pub async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let domain_socket_path = args.socket;

    Toplevel::new(|s| async move {
        s.start(SubsystemBuilder::new("Main", |s| async move {
            let result: Result<(), _> = match args.command {
                IndexerCommand::Client(cli) => cli.run(domain_socket_path).await,
                IndexerCommand::Server { server_command } => {
                    server_command.run(s, domain_socket_path).await
                }
                IndexerCommand::DbVersion => {
                    let version = IndexerStoreVersion::default();
                    let msg = serde_json::to_string(&version)?;
                    println!("{msg}");
                    return Ok(());
                }
                IndexerCommand::RestoreSnapshot {
                    snapshot_file_path,
                    restore_dir,
                } => {
                    info!("Received restore-snapshot with file {snapshot_file_path:#?} and dir {restore_dir:#?}");
                    let msg = if !snapshot_file_path.exists() {
                        let msg = format!("{snapshot_file_path:#?} does not exist");
                        error!("{msg}");
                        msg
                    } else if restore_dir.is_dir() {
                        // TODO: allow prompting user to overwrite
                        let msg = format!("{restore_dir:#?} must not exist (but currently does)");
                        error!("{msg}");
                        msg
                    } else {
                        let result = store::restore_snapshot(&snapshot_file_path, &restore_dir);
                        if result.is_ok() {
                            result?
                        } else {
                            #[allow(clippy::unnecessary_unwrap)]
                            let err = result.unwrap_err();
                            format!("{}: {:#?}", err, err.root_cause().to_string())
                        }
                    };
                    println!("{msg}");
                    return Ok(());
                }
            };
            result
        }));
    })
    .catch_signals()
    .handle_shutdown_requests(Duration::from_millis(1000))
    .await
    .map_err(Into::into)
}

impl ServerCommand {
    async fn run(self, subsys: SubsystemHandle, domain_socket_path: PathBuf) -> anyhow::Result<()> {
        use ServerCommand::*;
        let (args, mut mode) = match self {
            Shutdown => return client::ClientCli::Shutdown.run(domain_socket_path).await,
            Start(args) => (args, InitializationMode::New),
            Sync(args) => (args, InitializationMode::Sync),
            Replay(args) => (args, InitializationMode::Replay),
            StartViaConfig(args) => {
                let contents = std::fs::read(args.path.expect("server args config file"))?;
                let args: ServerArgsJson = serde_json::from_slice(&contents)?;
                (args.into(), InitializationMode::New)
            }
        };
        let args = args.with_dynamic_defaults(std::process::id());
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

        check_or_write_pid_file(&database_dir);

        debug!("Building an indexer configuration");
        let config = process_indexer_configuration(args, mode, domain_socket_path)?;

        debug!("Creating a new IndexerStore in {}", database_dir.display());
        let db = Arc::new(IndexerStore::new(&database_dir)?);

        let state = db.clone();
        subsys.start(SubsystemBuilder::new("Indexer", move |s| {
            start_indexer(s, config, state)
        }));

        info!(
            "Starting the web server listening on {}:{}",
            web_hostname, web_port
        );

        let state = db.clone();
        subsys.start(SubsystemBuilder::new("Web Server", move |s| {
            start_web_server(s, state, (web_hostname, web_port))
        }));

        info!("Shutting down primary database instance");
        db.database.cancel_all_background_work(true);
        drop(db);
        Ok(())
    }
}

fn process_indexer_configuration(
    args: ServerArgs,
    mode: InitializationMode,
    domain_socket_path: PathBuf,
) -> anyhow::Result<IndexerConfiguration> {
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

    let genesis_ledger = if let Some(ledger) = args.genesis_ledger {
        assert!(
            ledger.is_file(),
            "Ledger file does not exist at {}",
            ledger.display()
        );
        info!("Parsing ledger file at {}", ledger.display());

        match ledger::genesis::parse_file(&ledger) {
            Err(err) => {
                error!("Unable to parse genesis ledger: {err}");
                std::process::exit(100)
            }
            Ok(genesis_root) => {
                info!(
                    "Successfully parsed {} genesis ledger",
                    genesis_root.ledger.name
                );
                genesis_root.into()
            }
        }
    } else {
        let genesis_root =
            GenesisRoot::from_str(GenesisLedger::MAINNET_V1_GENESIS_LEDGER_CONTENTS)?;
        info!("Using default {} genesis ledger", genesis_root.ledger.name);
        genesis_root.into()
    };

    Ok(IndexerConfiguration {
        genesis_ledger,
        genesis_hash,
        genesis_constants,
        constraint_system_digests,
        version: PcbVersion::V1,
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

fn check_or_write_pid_file(database_dir: &Path) {
    use mina_indexer::platform;
    use std::{fs::File, io::Write, process};

    let _ = fs::create_dir_all(database_dir);

    let pid_path = database_dir.join("PID");

    if let Ok(pid) = fs::read_to_string(&pid_path) {
        let pid = pid
            .trim()
            .parse::<i32>()
            .unwrap_or_else(|_| panic!("Expected to find PID in {pid_path:#?}"));
        if platform::is_process_running(pid) {
            eprintln!("Will not start due to a running Indexer with PID {pid}");
            process::exit(130);
        }
        return;
    };

    match File::create(&pid_path) {
        Ok(mut pid_file) => {
            let pid = process::id();
            if let Err(e) = write!(pid_file, "{}", pid) {
                eprintln!("Error writing PID ({pid}) to {pid_path:#?}: {}", e);
                process::exit(131);
            }
        }
        Err(e) => {
            eprintln!("Error writing PID to {pid_path:#?}: {}", e);
            process::exit(131);
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ServerArgsJson {
    genesis_ledger: Option<String>,
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
    web_hostname: String,
    web_port: u16,
    pid: Option<u32>,
    missing_block_recovery_exe: Option<String>,
    missing_block_recovery_delay: Option<u64>,
    missing_block_recovery_batch: Option<bool>,
    network: String,
}

impl From<ServerArgs> for ServerArgsJson {
    fn from(value: ServerArgs) -> Self {
        let pid = value.pid.unwrap();
        let value = value.with_dynamic_defaults(pid);
        Self {
            genesis_ledger: value.genesis_ledger.map(|path| path.display().to_string()),
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
            web_hostname: value.web_hostname,
            web_port: value.web_port,
            pid: value.pid,
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
            genesis_ledger: value.genesis_ledger.and_then(|path| path.parse().ok()),
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
            web_hostname: value.web_hostname,
            web_port: value.web_port,
            pid: value.pid,
            missing_block_recovery_delay: value.missing_block_recovery_delay,
            missing_block_recovery_exe: value.missing_block_recovery_exe.map(|p| p.into()),
            missing_block_recovery_batch: value.missing_block_recovery_batch,
            network: (&value.network as &str).into(),
        }
    }
}
