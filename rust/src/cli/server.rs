use super::{database::DatabaseArgs, LogLevelFilter};
use crate::constants::*;
use std::{path::PathBuf, str::FromStr};

#[derive(clap::Parser, Debug, Clone, Default)]
#[command(author, version, about, long_about = None)]
pub struct ServerArgs {
    #[clap(flatten)]
    pub db: DatabaseArgs,

    /// Web server hostname for REST and GraphQL
    #[arg(long, default_value = DEFAULT_WEB_HOSTNAME)]
    pub web_hostname: String,

    /// Web server port for REST and GraphQL
    #[arg(long, default_value_t = DEFAULT_WEB_PORT)]
    pub web_port: u16,

    /// Start with data consistency checks
    #[arg(long, default_value_t = false)]
    pub self_check: bool,

    /// Path to the fetch new blocks executable
    #[arg(long)]
    pub fetch_new_blocks_exe: Option<PathBuf>,

    /// Delay (sec) in between fetch new blocks attempts
    #[arg(long)]
    pub fetch_new_blocks_delay: Option<u64>,

    /// Path to the missing block recovery executable
    #[arg(long)]
    pub missing_block_recovery_exe: Option<PathBuf>,

    /// Delay (sec) in between missing block recovery attempts
    #[arg(long)]
    pub missing_block_recovery_delay: Option<u64>,

    /// Recover all blocks at all missing heights
    #[arg(long)]
    pub missing_block_recovery_batch: Option<bool>,

    /// Indexer process ID
    #[arg(last = true)]
    pub pid: Option<u32>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ServerArgsJson {
    pub genesis_ledger: Option<String>,
    pub genesis_hash: String,
    pub genesis_constants: Option<String>,
    pub constraint_system_digests: Option<Vec<String>>,
    pub protocol_txn_version_digest: Option<String>,
    pub protocol_network_version_digest: Option<String>,
    pub blocks_dir: Option<String>,
    pub staking_ledgers_dir: Option<String>,
    pub database_dir: String,
    pub log_level: String,
    pub ledger_cadence: u32,
    pub reporting_freq: u32,
    pub prune_interval: u32,
    pub canonical_threshold: u32,
    pub canonical_update_threshold: u32,
    pub web_hostname: String,
    pub web_port: u16,
    pub pid: Option<u32>,
    pub do_not_ingest_orphan_blocks: bool,
    pub fetch_new_blocks_exe: Option<String>,
    pub fetch_new_blocks_delay: Option<u64>,
    pub missing_block_recovery_exe: Option<String>,
    pub missing_block_recovery_delay: Option<u64>,
    pub missing_block_recovery_batch: Option<bool>,
    pub network: String,
}

//////////
// impl //
//////////

impl ServerArgs {
    pub fn with_dynamic_defaults(mut self, pid: u32) -> Self {
        self.pid = Some(pid);
        self
    }
}

/////////////////
// conversions //
/////////////////

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
            genesis_constants: value.genesis_constants.map(Into::into),
            protocol_txn_version_digest: value.protocol_txn_version_digest,
            protocol_network_version_digest: value.protocol_network_version_digest,
            constraint_system_digests: value.constraint_system_digests,
            blocks_dir: value.blocks_dir.map(Into::into),
            staking_ledgers_dir: value.staking_ledgers_dir.map(Into::into),
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
            fetch_new_blocks_exe: value.fetch_new_blocks_exe.map(Into::into),
            missing_block_recovery_delay: value.missing_block_recovery_delay,
            missing_block_recovery_exe: value.missing_block_recovery_exe.map(Into::into),
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
