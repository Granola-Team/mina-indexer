use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore, BlockHash},
    staking_ledger::{staking_ledger_store::StakingLedgerStore, StakingLedger},
    state::{
        ledger::{store::LedgerStore, Ledger},
        snapshot::{StateSnapshot, StateStore},
        Canonicity,
    },
};
use mina_serialization_types::{
    signatures::SignatureJson, staged_ledger_diff::UserCommand, v1::UserCommandWithStatusV1,
};
use rocksdb::{
    backup::{BackupEngine, BackupEngineOptions, RestoreOptions},
    ColumnFamilyDescriptor, DBIterator, DB,
};
use std::{
    fs::remove_dir_all,
    path::{Path, PathBuf},
    str::FromStr,
};
use tracing::{info, instrument, trace};
use zstd::DEFAULT_COMPRESSION_LEVEL;

/// T-{Height}-{Timestamp}-{Signature} -> Transaction
/// We use the signature as key until we have a better way to identify transactions (e.g. hash)
/// The height is padded to 12 digits for sequential iteration
#[derive(Debug, Clone)]
pub struct TransactionKey(u32, u64, String);

impl std::fmt::Display for TransactionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "T-{:012}-{}-{}", self.0, self.1, self.2)
    }
}

impl TransactionKey {
    /// Creates a new key for a transaction
    pub fn new<S>(h: u32, t: u64, s: S) -> Self
    where
        S: Into<String>,
    {
        Self(h, t, s.into())
    }

    /// Returns the key as bytes
    pub fn bytes(&self) -> Vec<u8> {
        self.to_string().into_bytes()
    }

    /// Creates a new key from a slice
    pub fn from_slice(bytes: &[u8]) -> anyhow::Result<Self> {
        let key = std::str::from_utf8(bytes)?;
        let parts: Vec<&str> = key.split('-').collect();

        if parts.len() != 4 {
            anyhow::bail!("Invalid transaction key: {}", key);
        }

        Ok(Self(
            u32::from_str(parts[1])?,
            u64::from_str(parts[2])?,
            parts[3].to_string(),
        ))
    }

    /// Returns the height of the transaction
    pub fn height(&self) -> u32 {
        self.0
    }

    /// Returns the timestamp of the transaction
    pub fn timestamp(&self) -> u64 {
        self.1
    }

    /// Returns the signature of the transaction
    pub fn signature(&self) -> &str {
        &self.2
    }
}

#[derive(Debug)]
pub struct IndexerStore {
    db_path: PathBuf,
    database: DB,
}

impl IndexerStore {
    pub fn new_read_only(path: &Path, secondary: &Path) -> anyhow::Result<Self> {
        let database_opts = rocksdb::Options::default();
        let database = rocksdb::DBWithThreadMode::open_cf_as_secondary(
            &database_opts,
            path,
            secondary,
            vec!["blocks", "ledgers"],
        )?;
        Ok(Self {
            db_path: PathBuf::from(path),
            database,
        })
    }
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        let mut cf_opts = rocksdb::Options::default();
        cf_opts.set_max_write_buffer_number(16);
        let blocks = ColumnFamilyDescriptor::new("blocks", cf_opts.clone());
        let ledgers = ColumnFamilyDescriptor::new("ledgers", cf_opts.clone());
        let canonicity = ColumnFamilyDescriptor::new("canonicity", cf_opts.clone());
        let tx = ColumnFamilyDescriptor::new("tx", cf_opts);

        let mut database_opts = rocksdb::Options::default();
        database_opts.create_missing_column_families(true);
        database_opts.create_if_missing(true);
        let database = rocksdb::DBWithThreadMode::open_cf_descriptors(
            &database_opts,
            path,
            vec![blocks, ledgers, canonicity, tx],
        )?;
        Ok(Self {
            db_path: PathBuf::from(path),
            database,
        })
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn put_tx(
        &self,
        height: u32,
        timestamp: u64,
        tx: UserCommandWithStatusV1,
    ) -> anyhow::Result<()> {
        let cf_handle = self.database.cf_handle("tx").expect("column family exists");

        match tx.clone().inner().data.inner().inner() {
            UserCommand::SignedCommand(cmd) => {
                let json_sig = SignatureJson::from(cmd.inner().inner().signature);
                let sig = serde_json::to_string(&json_sig)?;

                let key = TransactionKey::new(height, timestamp, sig).bytes();
                let value = bcs::to_bytes(&tx)?;

                self.database.put_cf(&cf_handle, key, value)?;

                Ok(())
            }
        }
    }

    /// Creates a prefix iterator over a CF in the DB
    pub fn iter_prefix_cf(&self, cf: &str, prefix: &[u8]) -> DBIterator<'_> {
        let cf_handle = self.database.cf_handle(cf).expect("column family exists");
        self.database.prefix_iterator_cf(&cf_handle, prefix)
    }

    #[instrument]
    pub fn create_backup<BackupPath, BackupName>(
        &self,
        backup_name: BackupName,
        backup_path: BackupPath,
    ) -> anyhow::Result<()>
    where
        BackupPath: AsRef<Path> + std::fmt::Debug,
        BackupName: AsRef<str> + std::fmt::Debug,
    {
        info!(
            "creating backup with name {:?} in {:?} of rocksdb database in {:?}",
            backup_name.as_ref(),
            backup_path.as_ref(),
            &self.db_path
        );

        let mut backup_dir = PathBuf::from(backup_path.as_ref());
        backup_dir.push("rocksdb_backup");
        let mut snapshot_file_path = PathBuf::from(backup_path.as_ref());
        snapshot_file_path.push(&format!("{}.tar.zst", backup_name.as_ref()));

        trace!("initializing RocksDB backup engine in {backup_dir:?}");
        let backup_engine_options = BackupEngineOptions::new(&backup_dir)?;
        let backup_env = rocksdb::Env::new()?;
        let mut backup_engine = BackupEngine::open(&backup_engine_options, &backup_env)?;

        trace!("flushing database operations to disk and creating new RocksDB backup");
        backup_engine.create_new_backup_flush(&self.database, true)?;

        trace!(
            "creating backup tarball with name {:?}",
            backup_name.as_ref()
        );
        let backup_tarball = std::fs::File::create(&snapshot_file_path)?;
        let encoder = zstd::Encoder::new(backup_tarball, DEFAULT_COMPRESSION_LEVEL)?;
        let mut tar = tar::Builder::new(encoder);
        tar.append_dir_all("rocksdb_backup", &backup_dir)?;

        trace!("backup creation successful! cleaning up...");
        drop(tar.into_inner()?.finish()?);
        remove_dir_all(&backup_dir)?;

        Ok(())
    }

    #[instrument]
    pub fn from_backup<DebugPath>(
        backup_file: DebugPath,
        database_directory: DebugPath,
    ) -> anyhow::Result<Self>
    where
        DebugPath: AsRef<Path> + std::fmt::Debug, // I wish you could add a constraint here like Constraint<IsFile> or Constraint<IsDirectory>
    {
        info!(
            "restoring RocksDB database to {:?} from backup at {:?}",
            database_directory.as_ref(),
            backup_file.as_ref()
        );
        let mut backup_engine_path = PathBuf::from(backup_file.as_ref());
        backup_engine_path.pop();
        backup_engine_path.push("rocksdb_backup");
        let backup_engine_path = backup_engine_path;

        trace!(
            "unpacking backup data from {:?} to {:?}",
            backup_file.as_ref(),
            &backup_engine_path
        );
        let backup_tarball = std::fs::File::open(backup_file.as_ref())?;
        let zstd_decoder = zstd::Decoder::new(backup_tarball)?;
        let mut tar = tar::Archive::new(zstd_decoder);
        tar.unpack(&backup_engine_path)?;

        trace!(
            "initializing RocksDB backup engine in {:?}",
            &backup_engine_path
        );
        let backup_engine_options = BackupEngineOptions::new(&backup_engine_path)?;
        let backup_engine_env = rocksdb::Env::new()?;
        let mut backup_engine = BackupEngine::open(&backup_engine_options, &backup_engine_env)?;

        trace!(
            "restoring RocksDB backup from {:?} to database directory at {:?}",
            &backup_engine_path,
            database_directory.as_ref()
        );
        backup_engine.restore_from_latest_backup(
            database_directory.as_ref(),
            database_directory.as_ref(),
            &RestoreOptions::default(),
        )?;
        drop(backup_engine);

        trace!(
            "initializing IndexerStore with restored database at {:?}",
            database_directory.as_ref()
        );
        let indexer_store = IndexerStore::new(database_directory.as_ref())?;

        trace!("backup restoration completed successfully! cleaning up...");
        std::fs::remove_dir_all(&backup_engine_path)?;

        Ok(indexer_store)
    }
}

impl BlockStore for IndexerStore {
    fn add_block(&self, block: &PrecomputedBlock) -> anyhow::Result<()> {
        let cf_handle = self
            .database
            .cf_handle("blocks")
            .expect("column family exists");
        let key = block.state_hash.as_bytes();
        let value = bcs::to_bytes(&block)?;
        self.database.put_cf(&cf_handle, key, value)?;
        Ok(())
    }

    fn get_block(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PrecomputedBlock>> {
        let cf_handle = self
            .database
            .cf_handle("blocks")
            .expect("column family exists");
        let mut precomputed_block = None;
        self.database.try_catch_up_with_primary().ok();
        let key = state_hash.0.as_bytes();
        if let Some(bytes) = self
            .database
            .get_pinned_cf(&cf_handle, key)?
            .map(|bytes| bytes.to_vec())
        {
            precomputed_block = Some(bcs::from_bytes(&bytes)?);
        }
        Ok(precomputed_block)
    }

    fn set_canonicity(&self, state_hash: &BlockHash, canonicity: Canonicity) -> anyhow::Result<()> {
        if let Some(precomputed_block) = self.get_block(state_hash)? {
            let with_canonicity = PrecomputedBlock {
                canonicity: Some(canonicity),
                ..precomputed_block
            };
            self.add_block(&with_canonicity)?;
        }
        Ok(())
    }

    fn get_canonicity(&self, state_hash: &BlockHash) -> anyhow::Result<Option<Canonicity>> {
        let mut canonicity = None;
        if let Some(PrecomputedBlock {
            canonicity: Some(block_canonicity),
            ..
        }) = self.get_block(state_hash)?
        {
            canonicity = Some(block_canonicity);
        }
        Ok(canonicity)
    }
}

impl LedgerStore for IndexerStore {
    fn add_ledger(&self, state_hash: &BlockHash, ledger: Ledger) -> anyhow::Result<()> {
        let cf_handle = self
            .database
            .cf_handle("ledgers")
            .expect("column family exists");
        let key = state_hash.0.as_bytes();
        let value = bcs::to_bytes(&ledger)?;
        self.database.put_cf(&cf_handle, key, value)?;
        Ok(())
    }

    fn get_ledger(&self, state_hash: &BlockHash) -> anyhow::Result<Option<Ledger>> {
        let mut ledger = None;
        let key = state_hash.0.as_bytes();
        let cf_handle = self
            .database
            .cf_handle("ledgers")
            .expect("column family exists");

        self.database.try_catch_up_with_primary().ok();

        if let Some(bytes) = self
            .database
            .get_pinned_cf(&cf_handle, key)?
            .map(|bytes| bytes.to_vec())
        {
            ledger = Some(bcs::from_bytes(&bytes)?);
        }
        Ok(ledger)
    }
}

impl StakingLedgerStore for IndexerStore {
    fn add_epoch(&self, epoch: u32, ledger: &StakingLedger) -> anyhow::Result<()> {
        let cf_handle = self
            .database
            .cf_handle("epochs")
            .expect("column family exists");

        let key = epoch.to_be_bytes();
        let value = bcs::to_bytes(ledger)?;

        self.database.put_cf(&cf_handle, key, value)?;
        Ok(())
    }

    fn get_epoch(&self, ledger_hash: &str) -> anyhow::Result<Option<StakingLedger>> {
        let mut ledger = None;
        let key = ledger_hash.as_bytes();
        let cf_handle = self
            .database
            .cf_handle("staking_ledgers")
            .expect("column family exists");

        self.database.try_catch_up_with_primary().ok();

        if let Some(bytes) = self
            .database
            .get_pinned_cf(&cf_handle, key)?
            .map(|bytes| bytes.to_vec())
        {
            ledger = Some(bcs::from_bytes(&bytes)?);
        }
        Ok(ledger)
    }
}

impl StateStore for IndexerStore {
    fn store_state_snapshot(&self, snapshot: &StateSnapshot) -> anyhow::Result<()> {
        let key = b"STATE";
        let value = bcs::to_bytes(snapshot)?;
        self.database.put(key, value)?;
        Ok(())
    }

    fn read_snapshot(&self) -> anyhow::Result<Option<StateSnapshot>> {
        let mut snapshot = None;
        if let Some(bytes) = self
            .database
            .get_pinned(b"STATE")?
            .map(|bytes| bytes.to_vec())
        {
            snapshot = Some(bcs::from_bytes(&bytes)?);
        }
        Ok(snapshot)
    }
}

impl IndexerStore {
    pub fn test_conn(&mut self) -> anyhow::Result<()> {
        self.database.put("test", "value")?;
        self.database.delete("test")?;
        Ok(())
    }

    pub fn db_stats(&self) -> String {
        self.database
            .property_value(rocksdb::properties::DBSTATS)
            .unwrap()
            .unwrap()
    }

    pub fn memtables_size(&self) -> String {
        self.database
            .property_value(rocksdb::properties::CUR_SIZE_ALL_MEM_TABLES)
            .unwrap()
            .unwrap()
    }

    pub fn estimate_live_data_size(&self) -> u64 {
        self.database
            .property_int_value(rocksdb::properties::ESTIMATE_LIVE_DATA_SIZE)
            .unwrap()
            .unwrap()
    }

    pub fn estimate_num_keys(&self) -> u64 {
        self.database
            .property_int_value(rocksdb::properties::ESTIMATE_NUM_KEYS)
            .unwrap()
            .unwrap()
    }

    pub fn cur_size_all_mem_tables(&self) -> u64 {
        self.database
            .property_int_value(rocksdb::properties::CUR_SIZE_ALL_MEM_TABLES)
            .unwrap()
            .unwrap()
    }
}
