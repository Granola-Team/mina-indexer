//! This module contains the implementations of all store traits for the
//! [IndexerStore]

pub mod account;
pub mod account_store_impl;
pub mod block_store_impl;
pub mod canonicity_store_impl;
pub mod chain_store_impl;
pub mod column_families;
pub mod column_families_impl;
pub mod event_store_impl;
pub mod fixed_keys;
pub mod internal_command_store_impl;
pub mod ledger_store_impl;
pub mod snark_store_impl;
pub mod user_command_store_impl;
pub mod username;
pub mod username_store_impl;

use self::{column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys};
use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    command::signed::SignedCommandWithData,
    ledger::public_key::PublicKey,
};
use anyhow::{anyhow, bail};
use speedb::{ColumnFamilyDescriptor, DBCompressionType, DBIterator, IteratorMode, DB};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug)]
pub struct IndexerStore {
    pub db_path: PathBuf,
    pub database: DB,
    pub is_primary: bool,
}

impl IndexerStore {
    /// Add the corresponding CF helper to [ColumnFamilyHelpers]
    const COLUMN_FAMILIES: [&'static str; 42] = [
        "account-balance",
        "account-balance-sort",
        "account-balance-updates",
        "block-production-pk-epoch", // [block_production_pk_epoch_cf]
        "block-production-pk-total", // [block_production_pk_total_cf]
        "block-production-epoch",    // [block_production_epoch_cf]
        "blocks-state-hash",
        "blocks-version",
        "blocks-global-slot-idx",
        "blocks-at-length",
        "blocks-at-slot",
        "block-height-to-slot", // [block_height_to_global_slot_cf]
        "block-slot-to-height", // [block_global_slot_to_height_cf]
        "block-parent-hash",    // [block_parent_hash_cf]
        "blockchain-length",    // [blockchain_length_cf]
        "coinbase-receivers",   // [coinbase_receiver_cf]
        "canonicity",
        "user-commands",
        "mainnet-commands-slot",
        "mainnet-cmds-txn-global-slot",
        "mainnet-internal-commands",
        "internal-commands-global-slot-idx", // []
        "events",
        "ledgers",
        "snarks",
        "snark-work-top-producers",
        "snark-work-top-producers-sort",
        "snark-work-fees",            // [snark_work_fees_cf]
        "chain-id-to-network",        // [chain_id_to_network_cf]
        "txn-from",                   // [txn_from_cf]
        "txn-to",                     // [txn_to_cf]
        "user-commands-epoch",        // [user_commands_epoch_cf]
        "user-commands-pk-epoch",     // [user_commands_pk_epoch_cf]
        "user-commands-pk-total",     // [user_commands_pk_total_cf]
        "internal-commands-epoch",    // [internal_commands_epoch_cf]
        "internal-commands-pk-epoch", // [internal_commands_pk_epoch_cf]
        "internal-commands-pk-total", // [internal_commands_pk_total_cf]
        "snarks-epoch",               // [snarks_epoch_cf]
        "snarks-pk-epoch",            // [snarks_pk_epoch_cf]
        "snarks-pk-total",            // [snarks_pk_total_cf]
        "usernames",                  // [username_cf]
        "staking-ledger-epoch",       // [staking_ledger_epoch_cf]
    ];

    /// Creates a new _primary_ indexer store
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        // check that all column families are included
        assert_eq!(Self::COLUMN_FAMILIES.len(), Self::NUM_COLUMN_FAMILIES);

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
        Ok(Self {
            is_primary: true,
            db_path: path.into(),
            database: speedb::DBWithThreadMode::open_cf_descriptors(
                &database_opts,
                path,
                column_families,
            )?,
        })
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

/// For [BlockStore]
fn global_slot_block_key(block: &PrecomputedBlock) -> Vec<u8> {
    let mut res = to_be_bytes(block.global_slot_since_genesis());
    res.append(&mut block.state_hash().to_bytes());
    res
}

/// For [LedgerStore]

/// [DBIterator] for balance-sorted accounts
/// - `{balance BE bytes}{pk bytes} -> _`
/// - `balance`: 8 bytes
pub fn account_balance_iterator<'a>(
    db: &'a Arc<IndexerStore>,
    mode: IteratorMode,
) -> DBIterator<'a> {
    db.database.iterator_cf(db.account_balance_sort_cf(), mode)
}

/// [EventStore] implementation

/// [CommandStore] implementation

const COMMAND_KEY_PREFIX: &str = "user-";

/// Creates a new user command (transaction) database key from a &String
fn user_command_db_key_str(str: &String) -> String {
    format!("{COMMAND_KEY_PREFIX}{str}")
}

/// Creates a new user command (transaction) database key from one &String
fn user_command_db_key(str: &String) -> Vec<u8> {
    user_command_db_key_str(str).into_bytes()
}

/// Creates a new user command (transaction) database key for a public key
fn user_command_db_key_pk(pk: &String, n: u32) -> Vec<u8> {
    format!("{}-{n}", user_command_db_key_str(pk)).into_bytes()
}

/// Returns a user command (transaction) block state hash from a database key
pub fn convert_user_command_db_key_to_block_hash(db_key: &[u8]) -> anyhow::Result<BlockHash> {
    let db_key_str = std::str::from_utf8(db_key)?;
    let stripped_key = db_key_str.strip_prefix(COMMAND_KEY_PREFIX);

    if let Some(stripped_key) = stripped_key {
        let split_key: Vec<&str> = stripped_key.splitn(2, '-').collect();

        if let Some(first_part) = split_key.first() {
            return Ok(BlockHash(first_part.to_string()));
        }
    }
    bail!("User command key does not start with '{COMMAND_KEY_PREFIX}': {db_key_str}")
}

/// [DBIterator] for blocks
/// - key: `{global slot BE bytes}{state hash bytes}`
/// - value: empty byte
///
/// Use [blocks_global_slot_idx_state_hash_from_key] to extract state hash
pub fn blocks_global_slot_idx_iterator<'a>(
    db: &'a Arc<IndexerStore>,
    mode: IteratorMode,
) -> DBIterator<'a> {
    db.database
        .iterator_cf(db.blocks_global_slot_idx_cf(), mode)
}

/// Extracts state hash from the iterator entry (key)
pub fn blocks_global_slot_idx_state_hash_from_key(key: &[u8]) -> anyhow::Result<String> {
    Ok(String::from_utf8(key[4..].to_vec())?)
}

/// [DBIterator] for user commands (transactions)
pub fn user_commands_iterator<'a>(db: &'a Arc<IndexerStore>, mode: IteratorMode) -> DBIterator<'a> {
    db.database.iterator_cf(db.commands_slot_mainnet_cf(), mode)
}

/// [DBIterator] for user commands by sender
pub fn txn_from_iterator<'a>(db: &'a Arc<IndexerStore>, mode: IteratorMode) -> DBIterator<'a> {
    db.database.iterator_cf(db.txn_from_cf(), mode)
}

/// [DBIterator] for user commands by receiver
pub fn txn_to_iterator<'a>(db: &'a Arc<IndexerStore>, mode: IteratorMode) -> DBIterator<'a> {
    db.database.iterator_cf(db.txn_to_cf(), mode)
}

/// Global slot number from `key` in [user_commands_iterator]
/// - keep the first 4 bytes
pub fn user_commands_iterator_global_slot(key: &[u8]) -> u32 {
    from_be_bytes(key[0..4].to_vec())
}

/// Transaction hash from `key` in [user_commands_iterator]
/// - discard the first 4 bytes
pub fn user_commands_iterator_txn_hash(key: &[u8]) -> anyhow::Result<String> {
    String::from_utf8(key[4..].to_vec()).map_err(|e| anyhow!("Error reading txn hash: {}", e))
}

/// [SignedCommandWithData] from `entry` in [user_commands_iterator]
pub fn user_commands_iterator_signed_command(
    value: &[u8],
) -> anyhow::Result<SignedCommandWithData> {
    Ok(serde_json::from_slice(value)?)
}

pub fn to_be_bytes(value: u32) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

pub fn from_be_bytes(bytes: Vec<u8>) -> u32 {
    const SIZE: usize = 4;
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
/// - `prefix`: balance, etc
/// - `suffix`: txn hash, public key, etc
fn u64_prefix_key(prefix: u64, suffix: &str) -> Vec<u8> {
    let mut bytes = prefix.to_be_bytes().to_vec();
    bytes.append(&mut suffix.as_bytes().to_vec());
    bytes
}

/// Key format for sorting txns by sender/receiver: `{pk}{slot}{hash}`
/// - pk:   55 bytes (public key)
/// - slot: 4 BE bytes
/// - hash: rem bytes (txn hash)
pub fn txn_sort_key(public_key: PublicKey, global_slot: u32, txn_hash: &str) -> Vec<u8> {
    let mut bytes = public_key.to_bytes();
    bytes.append(&mut to_be_bytes(global_slot));
    bytes.append(&mut txn_hash.as_bytes().to_vec());
    bytes
}

pub fn txn_sort_key_prefix(public_key: PublicKey, global_slot: u32) -> Vec<u8> {
    let mut bytes = public_key.to_bytes();
    bytes.append(&mut to_be_bytes(global_slot));
    bytes
}

pub fn txn_sort_key_pk(key: &[u8]) -> PublicKey {
    PublicKey::from_bytes(&key[..PublicKey::LEN]).expect("public key")
}

pub fn txn_sort_key_global_slot(key: &[u8]) -> u32 {
    from_be_bytes(key[PublicKey::LEN..(PublicKey::LEN + 4)].to_vec())
}

pub fn txn_sort_key_txn_hash(key: &[u8]) -> String {
    String::from_utf8(key[(PublicKey::LEN + 4)..].to_vec()).expect("txn hash")
}

/// [DBIterator] for snark work fees
pub fn snark_fees_iterator<'a>(db: &'a IndexerStore, mode: IteratorMode) -> DBIterator<'a> {
    db.database.iterator_cf(db.snark_work_fees_cf(), mode)
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
