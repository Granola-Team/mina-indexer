use crate::{chain::Network, cli::LogLevelFilter, constants::*};
use std::path::PathBuf;

#[derive(clap::Parser, Debug, Clone, Default)]
#[command(author, version, about, long_about = None)]
pub struct DatabaseArgs {
    /// Path to the genesis ledger (JSON)
    #[arg(long, value_name = "FILE")]
    pub genesis_ledger: Option<PathBuf>,

    /// Hash of the initial state
    #[arg(
        long,
        default_value = MAINNET_GENESIS_HASH
    )]
    pub genesis_hash: String,

    /// Path to the genesis constants (JSON)
    #[arg(long)]
    pub genesis_constants: Option<PathBuf>,

    /// Override the constraint system digests
    #[arg(long)]
    pub constraint_system_digests: Option<Vec<String>>,

    /// Override the protocol transaction version digest
    #[arg(long)]
    pub protocol_txn_version_digest: Option<String>,

    /// Override the protocol network version digest
    #[arg(long)]
    pub protocol_network_version_digest: Option<String>,

    /// Directory of precomputed blocks
    #[arg(long)]
    pub blocks_dir: Option<PathBuf>,

    /// Directory of staking ledgers
    #[arg(long)]
    pub staking_ledgers_dir: Option<PathBuf>,

    /// Path to directory for speedb
    #[arg(long, default_value = "/var/lib/mina-indexer/database")]
    pub database_dir: PathBuf,

    /// Max stderr log level
    #[arg(long, default_value_t = LogLevelFilter::default())]
    pub log_level: LogLevelFilter,

    /// Number of blocks to add to the canonical chain before persisting a
    /// ledger snapshot
    #[arg(long, default_value_t = LEDGER_CADENCE)]
    pub ledger_cadence: u32,

    /// Number of blocks to process before reporting progress
    #[arg(long, default_value_t = BLOCK_REPORTING_FREQ_NUM)]
    pub reporting_freq: u32,

    /// Interval for pruning the root branch
    #[arg(long, default_value_t = PRUNE_INTERVAL_DEFAULT)]
    pub prune_interval: u32,

    /// Threshold for determining the canonicity of a block
    #[arg(long, default_value_t = MAINNET_CANONICAL_THRESHOLD)]
    pub canonical_threshold: u32,

    /// Threshold for updating the canonical root/ledger
    #[arg(long, default_value_t = CANONICAL_UPDATE_THRESHOLD)]
    pub canonical_update_threshold: u32,

    /// Start from a config file (bypasses other args)
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Network name
    #[arg(long, default_value = Network::Mainnet)]
    pub network: Network,

    /// Switch to not ingest orphan blocks
    #[arg(long, default_value_t = false)]
    pub do_not_ingest_orphan_blocks: bool,
}
