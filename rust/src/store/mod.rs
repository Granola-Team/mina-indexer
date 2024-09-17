//! This module contains the implementations of all store traits for the
//! [IndexerStore]

// traits
pub mod column_families;
pub mod fixed_keys;
pub mod username;
pub mod version;

// impls
pub mod best_ledger_store_impl;
pub mod block_store_impl;
pub mod canonicity_store_impl;
pub mod chain_store_impl;
pub mod column_families_impl;
pub mod event_store_impl;
pub mod internal_command_store_impl;
pub mod snark_store_impl;
pub mod staged_ledger_store_impl;
pub mod staking_ledger_store_impl;
pub mod user_command_store_impl;
pub mod username_store_impl;
pub mod version_store_impl;

use self::fixed_keys::FixedKeys;
use anyhow::{anyhow, bail, Context};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use speedb::{ColumnFamilyDescriptor, DBCompressionType, DB};
use std::{
    fs::{self, read_dir, File},
    io::{self, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};
use version::{IndexerStoreVersion, VersionStore};

#[derive(Debug)]
pub struct IndexerStore {
    pub db_path: PathBuf,
    pub database: DB,
    pub is_primary: bool,
}

#[derive(Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct DbUpdate<T> {
    pub apply: Vec<T>,
    pub unapply: Vec<T>,
}

impl IndexerStore {
    /// Add the corresponding CF helper to [ColumnFamilyHelpers]
    /// & modify [IndexerStoreVersion] as needed!
    const COLUMN_FAMILIES: [&'static str; 101] = [
        //////////////////////
        // Blocks store CFs //
        //////////////////////
        "blocks-state-hash",
        "blocks-version",
        "blocks-at-length",
        "blocks-at-slot",
        "blocks-height",
        "blocks-global-slot",
        "blocks-parent-hash",
        "blocks-date-time",
        "blocks-epoch",
        "blocks-genesis-hash",
        "blocks-height-to-slots",
        "blocks-slot-to-heights",
        "blocks-height-sort",
        "blocks-global-slot-sort",
        "blocks-comparison",
        "blocks-coinbase-receiver",
        "blocks-creator",
        "block-creator-height-sort",
        "block-creator-slot-sort",
        "coinbase-receiver-height-sort",
        "coinbase-receiver-slot-sort",
        //////////////////////////
        // Canonicity store CFs //
        //////////////////////////
        "canonicity-length",
        "canonicity-slot",
        ////////////////////////////
        // User command store CFs //
        ////////////////////////////
        "user-commands",
        "user-commands-pk",
        "user-commands-pk-num",
        "user-commands-block",
        "user-commands-block-order",
        "user-commands-num-blocks",
        "user-commands-slot-sort",
        "user-commands-height-sort",
        "user-commands-to-global-slot",
        "user-commands-to-block-height",
        "user-command-state-hashes",
        // sorting user commands by sender/receiver
        "txn-from-slot-sort",
        "txn-from-height-sort",
        "txn-to-slot-sort",
        "txn-to-height-sort",
        ////////////////////////////////
        // Internal command store CFs //
        ////////////////////////////////
        "internal-commands",
        "internal-commands-block-num",
        "internal-commands-global-slot-sort",
        "internal-commands-block-height-sort",
        "internal-commands-pk",
        "internal-commands-pk-num",
        "internal-commands-pk-global-slot-sort",
        "internal-commands-pk-block-height-sort",
        /////////////////////
        // SNARK store CFs //
        /////////////////////
        "snarks",
        "snark-work-top-producers",
        "snark-work-top-producers-sort",
        "snark-work-fees",
        "snark-work-prover",
        "snark-work-prover-height",
        /////////////////////
        // Event store CFs //
        /////////////////////
        "events",
        ///////////////////////////
        // Best ledger store CFs //
        ///////////////////////////
        "best-ledger-accounts",
        "best-ledger-account-balance-sort",
        "best-ledger-account-num-delegations",
        "best-ledger-account-delegations",
        /////////////////////////////
        // Staged ledger store CFs //
        /////////////////////////////
        "staged-ledger-accounts",
        "staged-ledger-account-balance-sort",
        "staged-ledger-account-num-delegations",
        "staged-ledger-account-delegations",
        "staged-ledger-hash-to-block",
        "staged-ledger-persisted",
        "staged-ledger-accounts-min-block",
        "blocks-ledger-diff",
        "blocks-staged-ledger-hash",
        //////////////////////////////
        // Staking ledger store CFs //
        //////////////////////////////
        "staking-ledger-accounts",
        "staking-ledger-delegations",
        "staking-ledger-persisted",
        "staking-ledger-epoch-to-hash",
        "staking-ledger-hash-to-epoch",
        "staking-ledger-genesis-hash",
        "staking-ledger-total-currency",
        "staking-ledger-balance-sort",
        "staking-ledger-stake-sort",
        "staking-ledger-accounts-count-epoch",
        /////////////////////
        // Chain store CFs //
        /////////////////////
        "chain-id-to-network",
        ////////////////////////
        // Username store CFs //
        ////////////////////////
        "username-pk-num",
        "username-pk-index",
        "usernames-per-block",
        // block counts
        "block-production-pk-epoch",
        "block-production-pk-canonical-epoch",
        "block-production-pk-supercharged-epoch",
        "block-production-pk-total",
        "block-production-pk-canonical-total",
        "block-production-pk-supercharged-total",
        "block-production-epoch",
        "block-production-canonical-epoch",
        "block-production-supercharged-epoch",
        "block-snark-counts",
        "block-user-command-counts",
        "block-internal-command-counts",
        // user command counts
        "user-commands-epoch",
        "user-commands-pk-epoch",
        "user-commands-pk-total",
        // internal command counts
        "internal-commands-epoch",
        "internal-commands-pk-epoch",
        "internal-commands-pk-total",
        // SNARK counts
        "snarks-epoch",
        "snarks-pk-epoch",
        "snarks-pk-total",
    ];

    /// Creates a new _primary_ indexer store
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        let mut cf_opts = speedb::Options::default();
        cf_opts.set_write_buffer_size(256 * 1024 * 1024); // 256MB
        cf_opts.set_max_write_buffer_number(16); // 256Mb * 16 ~= 4GB
        cf_opts.set_max_background_jobs(8);
        cf_opts.set_compression_type(DBCompressionType::Zstd);

        let mut database_opts = speedb::Options::default();
        database_opts.set_compression_type(DBCompressionType::Zstd);
        database_opts.create_missing_column_families(true);
        database_opts.create_if_missing(true);

        let column_families: Vec<ColumnFamilyDescriptor> = Self::COLUMN_FAMILIES
            .iter()
            .map(|cf| ColumnFamilyDescriptor::new(*cf, cf_opts.clone()))
            .collect();
        let primary = Self {
            is_primary: true,
            db_path: path.into(),
            database: speedb::DBWithThreadMode::open_cf_descriptors(
                &database_opts,
                path,
                column_families,
            )?,
        };

        // set db version
        primary.set_db_version_with_git_commit(
            IndexerStoreVersion::MAJOR,
            IndexerStoreVersion::MINOR,
            IndexerStoreVersion::PATCH,
        )?;
        let version = primary.get_db_version().expect("db version exists");
        persist_indexer_version(&version, path)?;
        Ok(primary)
    }

    /// Create a snapshot of the Indexer store
    pub fn create_snapshot(&self, output_file: &Path) -> Result<String, anyhow::Error> {
        use speedb::checkpoint::Checkpoint;

        let mut snapshot_temp_dir = output_file.to_path_buf();
        snapshot_temp_dir.set_extension("tmp-snapshot");
        Checkpoint::new(&self.database)?
            .create_checkpoint(&snapshot_temp_dir)
            .map_err(|e| anyhow!("Error creating database snapshot: {e}"))
            .and_then(|_| {
                persist_indexer_version(&IndexerStoreVersion::default(), &snapshot_temp_dir)?;
                archive_directory(&snapshot_temp_dir, output_file)
                    .with_context(|| "Failed to archive database")
            })
            .and_then(|_| {
                fs::remove_dir_all(&snapshot_temp_dir)
                    .with_context(|| format!("Failed to remove directory {snapshot_temp_dir:#?})"))
            })
            .map(|_| format!("Snapshot created and saved as {output_file:#?}"))
    }

    /// Create a read-only instance of an indexer store
    pub fn read_only(primary: &Path, secondary: &Path) -> anyhow::Result<Self> {
        let mut cf_opts = speedb::Options::default();
        cf_opts.set_max_write_buffer_number(16);
        cf_opts.set_compression_type(DBCompressionType::Zstd);

        let mut database_opts = speedb::Options::default();
        database_opts.set_compression_type(DBCompressionType::Zstd);
        database_opts.create_missing_column_families(true);
        database_opts.create_if_missing(true);

        let column_families: Vec<ColumnFamilyDescriptor> = Self::COLUMN_FAMILIES
            .iter()
            .map(|cf| ColumnFamilyDescriptor::new(*cf, cf_opts.clone()))
            .collect();
        let read_only = Self {
            is_primary: false,
            db_path: secondary.into(),
            database: speedb::DBWithThreadMode::open_cf_descriptors_as_secondary(
                &database_opts,
                primary,
                secondary,
                column_families,
            )?,
        };
        Ok(read_only)
    }
}

/// Restore a snapshot of the Indexer store
pub fn restore_snapshot(snapshot_file: &PathBuf, restore_dir: &PathBuf) -> anyhow::Result<()> {
    if !snapshot_file.exists() {
        bail!("Snapshot file {snapshot_file:#?} does not exist")
    } else if restore_dir.is_dir() {
        bail!("Restore dir {restore_dir:#?} must not exist")
    } else {
        extract_archive_file(snapshot_file, restore_dir)
            .with_context(|| format!("Failed to extract archive file {snapshot_file:#?}"))
            .map(|_| info!(
                "Snapshot successfully restored. Start mina indexer using `mina-indexer server start --database-dir {}`",
                restore_dir.display()
            ))
    }
}

fn extract_archive_file(archive_file: &Path, output_dir: &Path) -> io::Result<()> {
    debug!(
        "Extracting {} to {}",
        archive_file.display(),
        output_dir.display()
    );
    fs::create_dir_all(output_dir)?;

    let mut archive = tar::Archive::new(BufReader::new(File::open(archive_file)?));
    archive.unpack(output_dir)
}

fn archive_directory(input_dir: impl AsRef<Path>, output_file: impl AsRef<Path>) -> io::Result<()> {
    debug!(
        "Compressing {} to {}",
        input_dir.as_ref().display(),
        output_file.as_ref().display()
    );

    let mut archive = tar::Builder::new(BufWriter::new(File::create(output_file)?));
    read_dir(input_dir)?
        .flatten()
        .filter(|entry| entry.file_type().map_or(false, |ft| ft.is_file()))
        .for_each(|file| {
            archive
                .append_path_with_name(file.path(), file.file_name())
                .ok();
        });

    archive.finish()
}

impl<T> std::fmt::Debug for DbUpdate<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "apply:   {:#?}\nunapply: {:#?}",
            self.apply, self.unapply
        )
    }
}

/// For [UserCommandStore]

const COMMAND_KEY_PREFIX: &str = "user-";

/// Creates a new user command (transaction) database key for a public key
fn user_command_db_key_pk(pk: &str, n: u32) -> Vec<u8> {
    format!("{COMMAND_KEY_PREFIX}{pk}{n}").into_bytes()
}

pub fn persist_indexer_version(
    indexer_version: &IndexerStoreVersion,
    path: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let mut versioned = path.as_ref().to_path_buf();
    versioned.push("INDEXER_VERSION");
    if !versioned.exists() {
        debug!("persisting INDEXER_VERSION in the database directory");
        let serialized = serde_json::to_string(indexer_version)?;
        let mut file = std::fs::File::create(versioned)?;
        file.write_all(serialized.as_bytes())?;
    } else {
        debug!("INDEXER_VERSION file exists. Checking for compatability");
    }
    Ok(())
}

impl FixedKeys for IndexerStore {}

impl IndexerStore {
    pub fn db_stats(&self) -> String {
        self.database
            .property_value(speedb::properties::DBSTATS)
            .unwrap()
            .unwrap()
    }

    pub fn memtables_size(&self) -> String {
        self.database
            .property_value(speedb::properties::CUR_SIZE_ALL_MEM_TABLES)
            .unwrap()
            .unwrap()
    }

    pub fn estimate_live_data_size(&self) -> u64 {
        self.database
            .property_int_value(speedb::properties::ESTIMATE_LIVE_DATA_SIZE)
            .unwrap()
            .unwrap()
    }

    pub fn estimate_num_keys(&self) -> u64 {
        self.database
            .property_int_value(speedb::properties::ESTIMATE_NUM_KEYS)
            .unwrap()
            .unwrap()
    }

    pub fn cur_size_all_mem_tables(&self) -> u64 {
        self.database
            .property_int_value(speedb::properties::CUR_SIZE_ALL_MEM_TABLES)
            .unwrap()
            .unwrap()
    }
}
