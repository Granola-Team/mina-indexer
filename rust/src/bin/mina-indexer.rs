use clap::{Parser, Subcommand};
use log::{debug, error, info, warn, LevelFilter};
use mina_indexer::{
    block::precomputed::PcbVersion,
    chain::Network,
    client,
    constants::*,
    ledger::{
        self,
        genesis::{GenesisConstants, GenesisLedger, GenesisRoot},
    },
    server::{
        initialize_indexer_database, start_indexer, IndexerConfiguration, InitializationMode,
    },
    store::{restore_snapshot, version::IndexerStoreVersion, IndexerStore},
    unix_socket_server::remove_unix_socket,
    web::start_web_server,
};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process,
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use stderrlog::{ColorChoice, Timestamp};
use tempfile::TempDir;
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
    /// Database commands
    Database {
        #[command(subcommand)]
        db_command: DatabaseCommand,
    },

    /// Server commands
    Server {
        #[command(subcommand)]
        server_command: ServerCommand,
    },

    /// Client commands
    #[clap(flatten)]
    Client(#[command(subcommand)] client::ClientCli),

    /// Mina indexer version
    Version,
}

#[derive(Subcommand, Debug)]
enum ServerCommand {
    /// Start a new mina indexer
    Start(Box<ServerArgs>),

    /// Shutdown the server
    Shutdown,
}

#[derive(Subcommand, Debug)]
enum DatabaseCommand {
    Ingest {
        /// Max stdout log level
        #[arg(long, default_value_t = LogLevelFilter::default())]
        log_level: LogLevelFilter,

        /// Full path to a mina indexer database directory.
        /// If null, snapshot a running indexer database.
        #[arg(long)]
        database_dir: Option<PathBuf>,

        /// Directory of precomputed blocks
        #[arg(long)]
        blocks_dir: Option<PathBuf>,
    },

    /// Create a new mina indexer database to use with `mina-indexer start`
    Create(Box<DatabaseArgs>),

    /// Create a snapshot of a mina indexer database
    Snapshot {
        /// Full path to the snapshot file to be created
        #[arg(long, default_value = "./snapshot")]
        output_path: PathBuf,

        /// Full path to a mina indexer database directory.
        /// If null, snapshot a running indexer database.
        #[arg(long)]
        database_dir: Option<PathBuf>,
    },

    /// Restore an indexer database from an archived snapshot file
    Restore {
        /// Full path to the archive snapshot file
        #[arg(long, default_value = "./snapshot")]
        snapshot_file: PathBuf,

        /// Full path to the database directory
        #[arg(long)]
        restore_dir: PathBuf,
    },

    /// Query mina indexer database version
    Version {
        /// Output JSON data
        #[arg(long)]
        json: bool,
    },
}

#[derive(Parser, Debug, Clone, Default)]
#[command(author, version, about, long_about = None)]
pub struct ServerArgs {
    #[clap(flatten)]
    db: DatabaseArgs,

    /// Web server hostname for REST and GraphQL
    #[arg(long, default_value = DEFAULT_WEB_HOSTNAME)]
    web_hostname: String,

    /// Web server port for REST and GraphQL
    #[arg(long, default_value_t = DEFAULT_WEB_PORT)]
    web_port: u16,

    /// Start with data consistency checks
    #[arg(long, default_value_t = false)]
    self_check: bool,

    /// Path to the fetch new blocks executable
    #[arg(long)]
    fetch_new_blocks_exe: Option<PathBuf>,

    /// Delay (sec) in between fetch new blocks attempts
    #[arg(long)]
    fetch_new_blocks_delay: Option<u64>,

    /// Path to the missing block recovery executable
    #[arg(long)]
    missing_block_recovery_exe: Option<PathBuf>,

    /// Delay (sec) in between missing block recovery attempts
    #[arg(long)]
    missing_block_recovery_delay: Option<u64>,

    /// Recover all blocks at all missing heights
    #[arg(long)]
    missing_block_recovery_batch: Option<bool>,

    /// Indexer process ID
    #[arg(last = true)]
    pid: Option<u32>,
}

#[derive(Parser, Debug, Clone, Default)]
#[command(author, version, about, long_about = None)]
pub struct DatabaseArgs {
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
    #[arg(long)]
    genesis_constants: Option<PathBuf>,

    /// Override the constraint system digests
    #[arg(long)]
    constraint_system_digests: Option<Vec<String>>,

    /// Override the protocol transaction version digest
    #[arg(long)]
    protocol_txn_version_digest: Option<String>,

    /// Override the protocol network version digest
    #[arg(long)]
    protocol_network_version_digest: Option<String>,

    /// Directory of precomputed blocks
    #[arg(long)]
    blocks_dir: Option<PathBuf>,

    /// Directory of staking ledgers
    #[arg(long)]
    staking_ledgers_dir: Option<PathBuf>,

    /// Path to directory for speedb
    #[arg(long, default_value = "/var/log/mina-indexer/database")]
    pub database_dir: PathBuf,

    /// Max stdout log level
    #[arg(long, default_value_t = LogLevelFilter::default())]
    pub log_level: LogLevelFilter,

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

    /// Start from a config file (bypasses other args)
    #[arg(long)]
    config: Option<PathBuf>,

    /// Network name
    #[arg(long, default_value = Network::Mainnet)]
    network: Network,

    /// Switch to not ingest orphan blocks
    #[arg(long, default_value_t = false)]
    do_not_ingest_orphan_blocks: bool,
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

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let domain_socket_path = args.socket;
    Toplevel::new(|s| async move {
        s.start(SubsystemBuilder::new("Main", |s| async move {
            match args.command {
                IndexerCommand::Client(cli) => cli.run(domain_socket_path).await,
                IndexerCommand::Database { db_command } => db_command.run(domain_socket_path).await,
                IndexerCommand::Server { server_command } => {
                    server_command.run(s, domain_socket_path).await
                }
                IndexerCommand::Version => Ok(println!("{VERSION}")),
            }
        }));
    })
    .catch_signals()
    .handle_shutdown_requests(Duration::from_millis(1000))
    .await
    .map_err(anyhow::Error::from)
}

impl ServerCommand {
    async fn run(self, subsys: SubsystemHandle, domain_socket_path: PathBuf) -> anyhow::Result<()> {
        let (args, mode) = match self {
            Self::Shutdown => return client::ClientCli::Shutdown.run(domain_socket_path).await,
            Self::Start(args) => {
                if let Some(config_path) = args.db.config {
                    let contents = std::fs::read(config_path)?;
                    let args: ServerArgsJson = serde_json::from_slice(&contents)?;
                    (args.into(), InitializationMode::Sync)
                } else if args.self_check {
                    (*args, InitializationMode::Replay)
                } else {
                    (*args, InitializationMode::Sync)
                }
            }
        };
        let args = args.with_dynamic_defaults(std::process::id());
        let database_dir = args.db.database_dir.clone();
        let web_hostname = args.web_hostname.clone();
        let web_port = args.web_port;

        // initialize logging
        stderrlog::new()
            .module(module_path!())
            .color(ColorChoice::Never)
            .timestamp(Timestamp::Microsecond)
            .verbosity(args.db.log_level.0)
            .init()
            .unwrap();

        check_or_write_pid_file(&database_dir);

        debug!("Building mina indexer configuration");
        let config = process_indexer_configuration(args, mode, domain_socket_path.clone())?;
        let db = Arc::new(IndexerStore::new(&database_dir)?);

        info!("Starting the mina indexer filesystem watchers & UDS server");
        let store = db.clone();
        subsys.start(SubsystemBuilder::new("Indexer", move |s| {
            start_indexer(s, config, store)
        }));

        info!("Starting the web server listening on {web_hostname}:{web_port}");
        let store = db.clone();
        let host = web_hostname.clone();
        subsys.start(SubsystemBuilder::new("Web Server", move |s| {
            start_web_server(s, store, (host, web_port))
        }));

        println!("GraphQL server started at: http://{web_hostname}:{web_port}/graphql");
        subsys.on_shutdown_requested().await;

        info!("Shutting down primary database instance");
        db.database.cancel_all_background_work(true);
        remove_pid(&database_dir);
        drop(db);
        remove_unix_socket(&domain_socket_path)?;

        Ok(())
    }
}

impl DatabaseCommand {
    async fn run(self, domain_socket_path: PathBuf) -> anyhow::Result<()> {
        // initialize logging
        stderrlog::new()
            .module(module_path!())
            .color(ColorChoice::Never)
            .timestamp(Timestamp::Microsecond)
            .verbosity(LevelFilter::Info)
            .init()
            .unwrap();

        match self {
            Self::Version { json } => {
                let version = IndexerStoreVersion::default();
                println!(
                    "{}",
                    if json {
                        serde_json::to_string(&version)?
                    } else {
                        version.to_string()
                    }
                )
            }
            Self::Snapshot {
                output_path,
                database_dir,
            } => {
                if let Some(database_dir) = database_dir {
                    if !database_dir.exists() {
                        error!("Database dir {database_dir:#?} does not exist");
                    } else {
                        info!("Creating snapshot of database dir {database_dir:#?}");
                        let tmp_dir = TempDir::new()?;
                        let db = IndexerStore::read_only(&database_dir, tmp_dir.as_ref())?;
                        db.create_snapshot(&output_path)?;
                    }
                } else {
                    info!("Creating snapshot of running mina indexer");
                    return client::ClientCli::CreateSnapshot { output_path }
                        .run(domain_socket_path)
                        .await;
                }
            }
            Self::Restore {
                snapshot_file,
                restore_dir,
            } => {
                info!("Restoring mina indexer database from snapshot file {snapshot_file:#?} to {restore_dir:#?}");
                restore_snapshot(&snapshot_file, &restore_dir).unwrap_or_else(|e| error!("{e}"))
            }
            Self::Ingest {
                log_level,
                database_dir,
                blocks_dir,
            } => {
                info!(
                    "Ingesting blocks from {blocks_dir:#?} into {database_dir:#?} ({log_level:#?})"
                )
            }
            Self::Create(args) => {
                let database_dir = args.database_dir.clone();
                debug!("Ensuring mina indexer database exists in {database_dir:#?}");
                if let Err(e) = fs::create_dir_all(&database_dir) {
                    error!("Failed to create database directory: {e}");
                    process::exit(1);
                }
                debug!("Building mina indexer configuration");
                let mut mode = InitializationMode::BuildDB;
                if let Ok(dir) = std::fs::read_dir(database_dir.clone()) {
                    if dir.count() > 0 {
                        mode = InitializationMode::Sync;
                    }
                };
                let config =
                    process_indexer_configuration((*args).into(), mode, domain_socket_path)?;
                let db = Arc::new(IndexerStore::new(&database_dir)?);
                let store = db.clone();

                tokio::select! {
                    // wait for SIGINT
                    _ = tokio::signal::ctrl_c() => {
                        info!("SIGINT received");
                        store.database.cancel_all_background_work(true);
                    }

                    // build the database
                    res = initialize_indexer_database(config, &store) => {
                        if let Err(e) = res {
                            error!("Failed to initialize indexer database: {e}");
                        };
                    }
                }
            }
        }
        Ok(())
    }
}

/// Creates directories, processes constants & parses genesis ledger.
/// Returns indexer config.
fn process_indexer_configuration(
    args: ServerArgs,
    mode: InitializationMode,
    domain_socket_path: PathBuf,
) -> anyhow::Result<IndexerConfiguration> {
    let genesis_hash = args.db.genesis_hash.into();
    let blocks_dir = args.db.blocks_dir;
    let staking_ledgers_dir = args.db.staking_ledgers_dir;
    let genesis_constants = args.db.genesis_constants;
    let constraint_system_digests = args.db.constraint_system_digests;
    let protocol_txn_version_digest = args.db.protocol_txn_version_digest;
    let protocol_network_version_digest = args.db.protocol_network_version_digest;
    let prune_interval = args.db.prune_interval;
    let canonical_threshold = args.db.canonical_threshold;
    let canonical_update_threshold = args.db.canonical_update_threshold;
    let ledger_cadence = args.db.ledger_cadence;
    let reporting_freq = args.db.reporting_freq;
    let do_not_ingest_orphan_blocks = args.db.do_not_ingest_orphan_blocks;
    let fetch_new_blocks_exe = args.fetch_new_blocks_exe;
    let fetch_new_blocks_delay = args.fetch_new_blocks_delay;
    let missing_block_recovery_exe = args.missing_block_recovery_exe;
    let missing_block_recovery_delay = args.missing_block_recovery_delay;
    let missing_block_recovery_batch = args.missing_block_recovery_batch.unwrap_or(false);

    // ensure blocks dir exists
    if let Some(ref blocks_dir) = blocks_dir {
        debug!("Ensuring blocks directory exists: {blocks_dir:#?}");
        if let Err(e) = fs::create_dir_all(blocks_dir) {
            error!("Failed to create blocks directory: {e}");
            process::exit(1);
        }
    }

    // ensure staking ledgers dir exists
    if let Some(ref staking_ledgers_dir) = staking_ledgers_dir {
        debug!("Ensuring staking ledgers directory exists: {staking_ledgers_dir:#?}");
        if let Err(e) = fs::create_dir_all(staking_ledgers_dir) {
            error!("Failed to create staging ledger directory: {e}");
            process::exit(1);
        }
    }

    // pick up protocol constants from the given file or use defaults
    let genesis_constants = protocol_constants(genesis_constants)?;
    let constraint_system_digests = constraint_system_digests.unwrap_or(
        MAINNET_CONSTRAINT_SYSTEM_DIGESTS
            .iter()
            .map(|x| x.to_string())
            .collect(),
    );
    let genesis_ledger = parse_genesis_ledger(args.db.genesis_ledger)?;
    Ok(IndexerConfiguration {
        genesis_ledger,
        genesis_hash,
        genesis_constants,
        constraint_system_digests,
        protocol_txn_version_digest,
        protocol_network_version_digest,
        version: PcbVersion::default(),
        blocks_dir,
        staking_ledgers_dir,
        prune_interval,
        canonical_threshold,
        canonical_update_threshold,
        initialization_mode: mode,
        ledger_cadence,
        reporting_freq,
        domain_socket_path,
        fetch_new_blocks_exe,
        fetch_new_blocks_delay,
        missing_block_recovery_exe,
        missing_block_recovery_delay,
        missing_block_recovery_batch,
        do_not_ingest_orphan_blocks,
    })
}

fn parse_genesis_ledger(path: Option<PathBuf>) -> anyhow::Result<GenesisLedger> {
    let genesis_ledger = if let Some(path) = path {
        assert!(path.is_file(), "Ledger file does not exist at {path:#?}");
        info!("Parsing ledger file at {path:#?}");
        match ledger::genesis::parse_file(&path) {
            Err(err) => {
                error!("Unable to parse genesis ledger: {err}");
                std::process::exit(100)
            }
            Ok(genesis_root) => {
                info!(
                    "Successfully parsed {} genesis ledger",
                    genesis_root.ledger.name,
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
    Ok(genesis_ledger)
}

fn protocol_constants(path: Option<PathBuf>) -> anyhow::Result<GenesisConstants> {
    let mut constants = GenesisConstants::default();
    if let Some(path) = path {
        if let Ok(ref contents) = std::fs::read(path) {
            if let Ok(override_constants) = serde_json::from_slice::<GenesisConstants>(contents) {
                constants.override_with(override_constants);
            } else {
                error!(
                    "Error parsing supplied protocol constants. Using default:\n{}",
                    serde_json::to_string_pretty(&constants)?
                )
            }
        } else {
            error!(
                "Error reading protocol constants file. Using default:\n{}",
                serde_json::to_string_pretty(&constants)?
            )
        }
    }
    Ok(constants)
}

/// Read the pid from a file
fn read_pid_from_file<P: AsRef<Path>>(pid_path: P) -> anyhow::Result<i32> {
    let content = fs::read_to_string(pid_path)?;
    let pid = content.trim().parse()?;
    Ok(pid)
}

/// Write the current pid to a file
fn write_pid_to_file<P: AsRef<Path>>(pid_path: P) -> anyhow::Result<()> {
    let mut pid_file = File::create(pid_path)?;
    let pid = process::id();
    write!(pid_file, "{pid}")?;
    Ok(())
}

/// Remove PID file located in the database directory
fn remove_pid<P: AsRef<Path>>(database_dir: P) {
    let pid_path = database_dir.as_ref().join("PID");
    if let Err(e) = fs::remove_file(pid_path) {
        warn!("Failed to remove PID file: {e}");
    }
}

/// Checks if the current process is the owner of the database by verifying the
/// presence of a PID file. If another process is already running as the owner
/// of the database, the function stops the indexer. Otherwise, it claims
/// ownership by writing the current process ID (PID) into the database
/// directory.
///
/// This function ensures that only one process can own and operate on the
/// database at a time, preventing multiple instances of the indexer from
/// running concurrently.
///
/// # Arguments
///
/// * `database_dir` - A reference to the path of the database directory where
///   the PID file will be located.
fn check_or_write_pid_file<P: AsRef<Path>>(database_dir: P) {
    use mina_indexer::platform;
    let database_dir = database_dir.as_ref();
    let pid_path = database_dir.join("PID");

    if let Err(e) = fs::create_dir_all(database_dir) {
        error!("Failed to create database directory in {database_dir:?}: {e}");
        process::exit(1);
    }

    if let Ok(pid) = read_pid_from_file(&pid_path) {
        if platform::is_process_running(pid) {
            error!("Will not start due to a running Indexer with PID {pid}");
            process::exit(130);
        }
    }

    if let Err(e) = write_pid_to_file(&pid_path) {
        error!("Error writing PID to {pid_path:?}: {e}");
        process::exit(131);
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ServerArgsJson {
    genesis_ledger: Option<String>,
    genesis_hash: String,
    genesis_constants: Option<String>,
    constraint_system_digests: Option<Vec<String>>,
    protocol_txn_version_digest: Option<String>,
    protocol_network_version_digest: Option<String>,
    blocks_dir: Option<String>,
    staking_ledgers_dir: Option<String>,
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
    do_not_ingest_orphan_blocks: bool,
    fetch_new_blocks_exe: Option<String>,
    fetch_new_blocks_delay: Option<u64>,
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
            genesis_ledger: value
                .db
                .genesis_ledger
                .map(|path| path.display().to_string()),
            genesis_hash: value.db.genesis_hash,
            genesis_constants: value.db.genesis_constants.map(|g| g.display().to_string()),
            constraint_system_digests: value.db.constraint_system_digests,
            protocol_txn_version_digest: value.db.protocol_txn_version_digest,
            protocol_network_version_digest: value.db.protocol_network_version_digest,
            blocks_dir: value.db.blocks_dir.map(|d| d.display().to_string()),
            staking_ledgers_dir: value
                .db
                .staking_ledgers_dir
                .map(|d| d.display().to_string()),
            database_dir: value.db.database_dir.display().to_string(),
            log_level: value.db.log_level.to_string(),
            ledger_cadence: value.db.ledger_cadence,
            reporting_freq: value.db.reporting_freq,
            prune_interval: value.db.prune_interval,
            canonical_threshold: value.db.canonical_threshold,
            canonical_update_threshold: value.db.canonical_update_threshold,
            web_hostname: value.web_hostname,
            web_port: value.web_port,
            pid: value.pid,
            fetch_new_blocks_delay: value.fetch_new_blocks_delay,
            fetch_new_blocks_exe: value.fetch_new_blocks_exe.map(|p| p.display().to_string()),
            missing_block_recovery_delay: value.missing_block_recovery_delay,
            missing_block_recovery_exe: value
                .missing_block_recovery_exe
                .map(|p| p.display().to_string()),
            missing_block_recovery_batch: value.missing_block_recovery_batch,
            network: value.db.network.to_string(),
            do_not_ingest_orphan_blocks: value.db.do_not_ingest_orphan_blocks,
        }
    }
}

impl From<ServerArgsJson> for ServerArgs {
    fn from(value: ServerArgsJson) -> Self {
        let db = DatabaseArgs {
            genesis_ledger: value.genesis_ledger.and_then(|path| path.parse().ok()),
            genesis_hash: value.genesis_hash,
            genesis_constants: value.genesis_constants.map(|g| g.into()),
            protocol_txn_version_digest: value.protocol_txn_version_digest,
            protocol_network_version_digest: value.protocol_network_version_digest,
            constraint_system_digests: value.constraint_system_digests,
            blocks_dir: value.blocks_dir.map(|d| d.into()),
            staking_ledgers_dir: value.staking_ledgers_dir.map(|d| d.into()),
            database_dir: value.database_dir.into(),
            log_level: LogLevelFilter::from_str(&value.log_level).expect("log level"),
            ledger_cadence: value.ledger_cadence,
            reporting_freq: value.reporting_freq,
            prune_interval: value.prune_interval,
            canonical_threshold: value.canonical_threshold,
            canonical_update_threshold: value.canonical_update_threshold,
            config: None,
            network: (&value.network as &str).into(),
            do_not_ingest_orphan_blocks: value.do_not_ingest_orphan_blocks,
        };
        Self {
            db,
            web_hostname: value.web_hostname,
            web_port: value.web_port,
            self_check: false,
            pid: value.pid,
            fetch_new_blocks_delay: value.fetch_new_blocks_delay,
            fetch_new_blocks_exe: value.fetch_new_blocks_exe.map(|p| p.into()),
            missing_block_recovery_delay: value.missing_block_recovery_delay,
            missing_block_recovery_exe: value.missing_block_recovery_exe.map(|p| p.into()),
            missing_block_recovery_batch: value.missing_block_recovery_batch,
        }
    }
}

impl From<DatabaseArgs> for ServerArgs {
    fn from(value: DatabaseArgs) -> Self {
        Self {
            db: value,
            web_hostname: DEFAULT_WEB_HOSTNAME.to_string(),
            web_port: DEFAULT_WEB_PORT,
            ..Default::default()
        }
    }
}

const DEFAULT_WEB_HOSTNAME: &str = "localhost";
const DEFAULT_WEB_PORT: u16 = 8080;

#[derive(Debug, Clone)]
pub struct LogLevelFilter(LevelFilter);

impl Default for LogLevelFilter {
    fn default() -> Self {
        Self(LevelFilter::Warn)
    }
}

impl std::fmt::Display for LogLevelFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for LogLevelFilter {
    type Err = <LevelFilter as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        LevelFilter::from_str(s).map(Self)
    }
}
