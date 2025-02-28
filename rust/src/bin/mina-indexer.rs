use clap::{Parser, Subcommand};
use log::{debug, error, info, warn, LevelFilter};
use mina_indexer::{
    block::precomputed::PcbVersion,
    chain::ChainId,
    cli::{
        database::DatabaseArgs,
        server::{ServerArgs, ServerArgsJson},
    },
    client,
    constants::*,
    ledger::genesis::GenesisLedger,
    server::{GenesisVersion, IndexerConfiguration, IndexerVersion, InitializationMode},
    store::{restore_snapshot, version::IndexerStoreVersion, IndexerStore},
    unix_socket_server::remove_unix_socket,
    web::start_web_server,
};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process,
    sync::Arc,
    time::Duration,
};
use stderrlog::{ColorChoice, Timestamp};
use tempfile::TempDir;
use tokio_graceful_shutdown::{
    errors::SubsystemError, SubsystemBuilder, SubsystemHandle, Toplevel,
};

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

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let domain_socket_path = args.socket;

    let result = Toplevel::new(|s| async move {
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
    .await;

    match result {
        Ok(_) => Ok(()),
        Err(shutdown_error) => {
            // Extract and log the specific error details
            let subsystem_errors = shutdown_error.get_subsystem_errors();

            if !subsystem_errors.is_empty() {
                let first_error = subsystem_errors.iter().next().unwrap();

                match first_error {
                    SubsystemError::Failed(_, error) => Err(anyhow::anyhow!("{}", error)),
                    SubsystemError::Panicked(name) => {
                        Err(anyhow::anyhow!("Subsystem '{}' panicked", name))
                    }
                }
            } else {
                Err(anyhow::anyhow!("{}", shutdown_error))
            }
        }
    }
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

        let config = process_indexer_configuration(args, mode, domain_socket_path.clone())?;

        info!("Starting the mina indexer filesystem watchers & UDS server");
        let db = Arc::new(IndexerStore::new(&database_dir)?);
        let store = db.clone();

        subsys.start(SubsystemBuilder::new("Indexer", move |s| {
            config.start_indexer(s, store)
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

                let config = if let Some(config_path) = args.config {
                    let contents = std::fs::read(config_path)?;
                    let args: ServerArgsJson = serde_json::from_slice(&contents)?;
                    IndexerConfiguration::from((args, domain_socket_path))
                } else {
                    process_indexer_configuration((*args).into(), mode, domain_socket_path)?
                };
                let db = Arc::new(IndexerStore::new(&database_dir)?);
                let store = db.clone();

                tokio::select! {
                    // wait for SIGINT
                    _ = tokio::signal::ctrl_c() => {
                        info!("SIGINT received");
                        store.database.cancel_all_background_work(true);
                    }

                    // build the database
                    res = config.initialize_indexer_database(&store) => {
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
    initialization_mode: InitializationMode,
    domain_socket_path: PathBuf,
) -> anyhow::Result<IndexerConfiguration> {
    let genesis_hash = args.db.genesis_hash;
    let blocks_dir = args.db.blocks_dir;
    let staking_ledgers_dir = args.db.staking_ledgers_dir;
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
    // let genesis_constants = args.db.genesis_constants;
    // let constraint_system_digests = args.db.constraint_system_digests;
    // let protocol_txn_version_digest = args.db.protocol_txn_version_digest;
    // let protocol_network_version_digest =
    // args.db.protocol_network_version_digest; let genesis_constants =
    // protocol_constants(genesis_constants)?; let constraint_system_digests =
    // constraint_system_digests.unwrap_or(
    //     MAINNET_CONSTRAINT_SYSTEM_DIGESTS
    //         .iter()
    //         .map(|x| x.to_string())
    //         .collect(),
    // );

    // indexer version
    let network = args.db.network;
    let (version, chain_id, genesis) = if genesis_hash == HARDFORK_GENESIS_HASH {
        (PcbVersion::V2, ChainId::v2(), GenesisVersion::v2())
    } else {
        (PcbVersion::V1, ChainId::v1(), GenesisVersion::v1())
    };

    let genesis_ledger = parse_genesis_ledger(args.db.genesis_ledger, &version)?;
    let version = IndexerVersion {
        network,
        version,
        chain_id,
        genesis,
    };

    Ok(IndexerConfiguration {
        genesis_ledger,
        version,
        blocks_dir,
        staking_ledgers_dir,
        prune_interval,
        canonical_threshold,
        canonical_update_threshold,
        initialization_mode,
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

fn parse_genesis_ledger(
    path: Option<PathBuf>,
    version: &PcbVersion,
) -> anyhow::Result<GenesisLedger> {
    let genesis_ledger = if let Some(path) = path {
        assert!(path.is_file(), "Ledger file does not exist at {path:#?}");
        info!("Parsing ledger file at {path:#?}");

        match GenesisLedger::parse_file(&path) {
            Err(err) => {
                error!("Unable to parse genesis ledger: {err}");
                std::process::exit(100)
            }
            Ok(genesis) => {
                info!("Successfully parsed genesis ledger");
                genesis
            }
        }
    } else {
        info!("Using default {} genesis ledger", version);
        match version {
            PcbVersion::V1 => GenesisLedger::new_v1()?,
            PcbVersion::V2 => GenesisLedger::new_v2()?,
        }
    };

    Ok(genesis_ledger)
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
