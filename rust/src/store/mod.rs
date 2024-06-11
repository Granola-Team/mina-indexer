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
use anyhow::anyhow;
use speedb::{ColumnFamilyDescriptor, DBCompressionType, DB};
use std::path::{Path, PathBuf};
use version::{IndexerStoreVersion, VersionStore};

#[derive(Debug)]
pub struct IndexerStore {
    pub db_path: PathBuf,
    pub database: DB,
    pub is_primary: bool,
}

impl IndexerStore {
    /// Add the corresponding CF helper to [ColumnFamilyHelpers]
    /// & modify [IndexerStoreVersion] as needed!
    const COLUMN_FAMILIES: [&'static str; 63] = [
        "account-balance",
        "account-balance-sort",
        "account-balance-updates",
        "block-production-pk-epoch",
        "block-production-pk-total",
        "block-production-epoch",
        "blocks-state-hash",
        "blocks-version",
        "blocks-at-length",
        "blocks-at-slot",
        "blocks-height",
        "blocks-height-sort",
        "blocks-global-slot",
        "blocks-global-slot-sort",
        "blocks-parent-hash",
        "block-height-to-slots",
        "block-slot-to-heights",
        "block-comparison",
        "coinbase-receivers",
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
        // user command -> list of state hashes
        "user-command-state-hashes",
        // sorting user commands by sender/receiver
        "txn-from-slot-sort",
        "txn-from-height-sort",
        "txn-to-slot-sort",
        "txn-to-height-sort",
        // internal commands
        "internal-commands",
        "internal-commands-global-slot",
        "events",
        "ledgers",
        // SNARKs
        "snarks",
        "snark-work-top-producers",
        "snark-work-top-producers-sort",
        "snark-work-fees",
        "snark-work-prover",
        "chain-id-to-network",
        // sorting staking ledgers
        "staking-ledger-epoch",
        "staking-ledger-balance",
        "staking-ledger-stake",
        // usernames
        "usernames",
        "usernames-per-block",
        // data counts
        "user-commands-epoch",
        "user-commands-pk-epoch",
        "user-commands-pk-total",
        "internal-commands-epoch",
        "internal-commands-pk-epoch",
        "internal-commands-pk-total",
        "snarks-epoch",
        "snarks-pk-epoch",
        "snarks-pk-total",
        "block-snark-counts",
        "block-user-command-counts",
        "block-internal-command-counts",
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
        Ok(primary)
    }

    pub fn create_checkpoint(&self, path: &Path) -> anyhow::Result<()> {
        use speedb::checkpoint::Checkpoint;

        let checkpoint = Checkpoint::new(&self.database)?;
        Checkpoint::create_checkpoint(&checkpoint, path)
            .map_err(|e| anyhow!("Error creating db checkpoint: {}", e))
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}

/// For [UserCommandStore]

const COMMAND_KEY_PREFIX: &str = "user-";

/// Creates a new user command (transaction) database key for a public key
fn user_command_db_key_pk(pk: &str, n: u32) -> Vec<u8> {
    format!("{COMMAND_KEY_PREFIX}{pk}{n}").into_bytes()
}

/// Extracts state hash from the iterator key.
/// Used with [blocks_height_iterator] & [blocks_global_slot_iterator]
pub fn block_state_hash_from_key(key: &[u8]) -> anyhow::Result<BlockHash> {
    BlockHash::from_bytes(&key[4..])
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

pub fn pk_of_key(key: &[u8]) -> PublicKey {
    PublicKey::from_bytes(&key[..PublicKey::LEN]).expect("public key")
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
