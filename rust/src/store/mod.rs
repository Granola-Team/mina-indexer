//! This module contains the implementations of all store traits for the
//! [IndexerStore]

// traits
pub mod account;
pub mod column_families;
pub mod fixed_keys;
pub mod username;
pub mod version;

// impls
pub mod account_store_impl;
pub mod block_store_impl;
pub mod canonicity_store_impl;
pub mod chain_store_impl;
pub mod column_families_impl;
pub mod event_store_impl;
pub mod internal_command_store_impl;
pub mod ledger_store_impl;
pub mod snark_store_impl;
pub mod user_command_store_impl;
pub mod username_store_impl;
pub mod version_store_impl;

use self::fixed_keys::FixedKeys;
use crate::{block::BlockHash, command::signed::TXN_HASH_LEN, ledger::public_key::PublicKey};
use anyhow::{anyhow, Context};
use chrono::{Datelike, Local};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use log::{debug, error};
use serde::{Deserialize, Serialize};
use speedb::{ColumnFamilyDescriptor, DBCompressionType, DB};
use std::{
    fs::{self, read_dir, File},
    io::{self, prelude::*, Write},
    path::{Path, PathBuf},
};
use tar::Archive;
use version::{IndexerStoreVersion, VersionStore};

#[derive(Debug)]
pub struct IndexerStore {
    pub db_path: PathBuf,
    pub database: DB,
    pub is_primary: bool,
}

#[derive(Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct DBUpdate<T> {
    pub apply: Vec<T>,
    pub unapply: Vec<T>,
}

impl IndexerStore {
    /// Add the corresponding CF helper to [ColumnFamilyHelpers]
    /// & modify [IndexerStoreVersion] as needed!
    const COLUMN_FAMILIES: [&'static str; 80] = [
        // accounts
        "account-balance",
        "account-balance-sort",
        "account-balance-updates",
        // blocks
        "blocks-state-hash",
        "blocks-version",
        "blocks-at-length",
        "blocks-at-slot",
        "blocks-height",
        "blocks-global-slot",
        "blocks-parent-hash",
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
        // canonicity
        "canonicity-length",
        "canonicity-slot",
        // user commands
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
        // SNARKs
        "snarks",
        "snark-work-top-producers",
        "snark-work-top-producers-sort",
        "snark-work-fees",
        "snark-work-prover",
        "snark-work-prover-height",
        // internal commands
        "internal-commands",
        "internal-commands-global-slot",
        // indexer store events
        "events",
        // staged ledgers
        "ledgers",
        "blocks-ledger-diff",
        "blocks-staged-ledger-hash",
        // staking ledgers & delegations
        "staking-ledgers",
        "staking-delegations",
        "staking-ledger-genesis-hash",
        "staking-ledger-epoch-to-hash",
        "staking-ledger-hash-to-epoch",
        // sorting staking ledgers
        "staking-ledger-epoch",
        "staking-ledger-balance",
        "staking-ledger-stake",
        "staking-ledger-accounts-epoch",
        // chain id
        "chain-id-to-network",
        // usernames
        "username-pk-num",
        "username-pk-index",
        "usernames-per-block",
        // block counts
        "block-production-pk-epoch",
        "block-production-pk-total",
        "block-production-epoch",
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
    pub fn create_snapshot(&self, input_dir: &PathBuf) -> Result<String, anyhow::Error> {
        use speedb::checkpoint::Checkpoint;

        if !input_dir.is_dir() {
            let msg = format!("{input_dir:#?} is not a directory");
            error!("{msg}");
            Err(anyhow!(msg))
        } else {
            let snapshot_temp_dir = input_dir.join("snapshot_temp");
            let result = Checkpoint::new(&self.database)?
                .create_checkpoint(&snapshot_temp_dir)
                .map_err(|e| anyhow!("Error creating database snapshot: {}", e));

            if result.is_ok() {
                let output_filename = format!(
                    "snapshot_{}_{}.tar.gz",
                    get_current_date(),
                    IndexerStoreVersion::default().major_minor_patch()
                );

                let output_filepath = input_dir.join(output_filename);

                let result = compress_directory(&snapshot_temp_dir, &output_filepath)
                    .with_context(|| format!("Failed to compress directory {input_dir:#?})"));
                if result.is_ok() {
                    let result = fs::remove_dir_all(&snapshot_temp_dir).with_context(|| {
                        format!("Failed to remove directory {snapshot_temp_dir:#?})")
                    });
                    if result.is_ok() {
                        return Ok(format!(
                            "Snapshot created and saved as {output_filepath:#?}"
                        ));
                    }
                }
            }
            #[allow(clippy::unnecessary_unwrap)]
            Err(result.unwrap_err())
        }
    }
}

/// Restore a snapshot of the Indexer store
pub fn restore_snapshot(
    snapshot_file_path: &PathBuf,
    restore_dir: &PathBuf,
) -> Result<String, anyhow::Error> {
    if !snapshot_file_path.exists() {
        let msg = format!("{snapshot_file_path:#?} does not exist");
        error!("{msg}");
        Err(anyhow!(msg))
    } else if restore_dir.is_dir() {
        let msg = format!("{restore_dir:#?} must not exist (but currently does)");
        error!("{msg}");
        Err(anyhow!(msg))
    } else {
        let result = decompress_file(snapshot_file_path, restore_dir)
            .with_context(|| format!("Failed to decompress file {snapshot_file_path:#?}"));
        if result.is_ok() {
            Ok(format!(
                "Snapshot restored to {restore_dir:#?}.\n\nPlease start server using: `server sync --database-dir {restore_dir:#?}`"
            ))
        } else {
            #[allow(clippy::unnecessary_unwrap)]
            Err(result.unwrap_err())
        }
    }
}

fn decompress_file(compressed_file_path: &PathBuf, output_dir: &PathBuf) -> io::Result<()> {
    fs::create_dir_all(output_dir)?;

    let compressed_file = File::open(compressed_file_path)?;
    let mut decoder = GzDecoder::new(compressed_file);

    let mut buffer = Vec::new();
    decoder.read_to_end(&mut buffer)?;

    let mut archive = Archive::new(buffer.as_slice());
    archive.unpack(output_dir)
}

fn compress_directory(input_dir: &PathBuf, output_file: &PathBuf) -> io::Result<()> {
    let output_file = File::create(output_file)?;
    let encoder = GzEncoder::new(output_file, Compression::default());

    let mut archive = tar::Builder::new(encoder);
    let dir = read_dir(input_dir)?;

    for entry in dir {
        let file = entry?.path();
        let file_name = &file.file_name().unwrap().to_str().unwrap();
        archive.append_file(file_name, &mut File::open(&file)?)?;
    }

    archive.finish()
}

// Get the current date as "YYYY-MM-DD"
fn get_current_date() -> String {
    let now = Local::now();
    format!("{:04}-{:02}-{:02}", now.year(), now.month(), now.day())
}

impl<T> std::fmt::Debug for DBUpdate<T>
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

/// Extracts state hash suffix from the iterator key.
/// Used with [blocks_height_iterator] & [blocks_global_slot_iterator]
pub fn block_state_hash_from_key(key: &[u8]) -> anyhow::Result<BlockHash> {
    BlockHash::from_bytes(&key[key.len() - BlockHash::LEN..])
}

/// Extracts u32 BE prefix from the iterator key.
/// Used with [blocks_height_iterator] & [blocks_global_slot_iterator]
pub fn block_u32_prefix_from_key(key: &[u8]) -> anyhow::Result<u32> {
    Ok(from_be_bytes(key[..4].to_vec()))
}

pub fn to_be_bytes(value: u32) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

pub fn from_be_bytes(bytes: Vec<u8>) -> u32 {
    const SIZE: usize = (u32::BITS / 8) as usize;
    let mut be_bytes = [0; SIZE];

    be_bytes[..SIZE].copy_from_slice(&bytes[..SIZE]);
    u32::from_be_bytes(be_bytes)
}

/// The first 4 bytes are `prefix` in big endian
/// - `prefix`: global slot, epoch number, etc
/// - `suffix`: txn hash, public key, etc
fn u32_prefix_key(prefix: u32, suffix: &str) -> Vec<u8> {
    let mut bytes = to_be_bytes(prefix);
    bytes.append(&mut suffix.as_bytes().to_vec());
    bytes
}

/// The first 8 bytes are `prefix` in big endian
/// ```
/// - prefix: balance, etc
/// - suffix: txn hash, public key, etc
fn u64_prefix_key(prefix: u64, suffix: &str) -> Vec<u8> {
    let mut bytes = prefix.to_be_bytes().to_vec();
    bytes.append(&mut suffix.as_bytes().to_vec());
    bytes
}

/// Key format for sorting txns by global slot:
/// `{u32_prefix}{txn_hash}{state_hash}`
/// ```
/// - u32_prefix: 4 BE bytes
/// - txn_hash:   [TXN_HASH_LEN] bytes
/// - state_hash: [BlockHash::LEN] bytes
pub fn txn_sort_key(prefix: u32, txn_hash: &str, state_hash: BlockHash) -> Vec<u8> {
    let mut bytes = to_be_bytes(prefix);
    bytes.append(&mut txn_hash.as_bytes().to_vec());
    bytes.append(&mut state_hash.to_bytes());
    bytes
}

/// Key format for sorting txns by sender/receiver:
/// `{pk}{u32_sort}{txn_hash}{state_hash}`
/// ```
/// - pk:         [PublicKey::LEN] bytes
/// - u32_sort:   4 BE bytes
/// - txn_hash:   [TXN_HASH_LEN] bytes
/// - state_hash: [BlockHash::LEN] bytes
pub fn pk_txn_sort_key(pk: PublicKey, sort: u32, txn_hash: &str, state_hash: BlockHash) -> Vec<u8> {
    let mut bytes = pk.to_bytes();
    bytes.append(&mut txn_sort_key(sort, txn_hash, state_hash));
    bytes
}

/// Prefix `{pk}{u32_sort}`
pub fn pk_txn_sort_key_prefix(public_key: PublicKey, sort: u32) -> Vec<u8> {
    let mut bytes = public_key.to_bytes();
    bytes.append(&mut to_be_bytes(sort));
    bytes
}

/// Parse the first [PublicKey::LEN]
pub fn pk_of_key(key: &[u8]) -> PublicKey {
    PublicKey::from_bytes(&key[..PublicKey::LEN]).expect("public key")
}

pub fn balance_of_key(key: &[u8]) -> u64 {
    let mut balance_bytes = [0; 8];
    balance_bytes.copy_from_slice(&key[..8]);
    u64::from_be_bytes(balance_bytes)
}

pub fn u32_of_key(key: &[u8]) -> u32 {
    from_be_bytes(key[PublicKey::LEN..(PublicKey::LEN + 4)].to_vec())
}

pub fn txn_hash_of_key(key: &[u8]) -> String {
    String::from_utf8(key[(PublicKey::LEN + 4)..(PublicKey::LEN + 4 + TXN_HASH_LEN)].to_vec())
        .expect("txn hash")
}

pub fn state_hash_pk_txn_sort_key(key: &[u8]) -> BlockHash {
    BlockHash::from_bytes(&key[(PublicKey::LEN + 4 + TXN_HASH_LEN)..]).expect("state hash")
}

pub fn block_txn_index_key(state_hash: &BlockHash, index: u32) -> Vec<u8> {
    let mut key = state_hash.clone().to_bytes();
    key.append(&mut to_be_bytes(index));
    key
}

pub fn txn_block_key(txn_hash: &str, state_hash: BlockHash) -> Vec<u8> {
    let mut bytes = txn_hash.as_bytes().to_vec();
    bytes.append(&mut state_hash.clone().to_bytes());
    bytes
}

pub fn persist_indexer_version(
    indexer_version: &IndexerStoreVersion,
    path: &Path,
) -> anyhow::Result<()> {
    let mut versioned = path.to_path_buf();
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
