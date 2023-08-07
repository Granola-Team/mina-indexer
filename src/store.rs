use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore, BlockHash, signed_command},
    staking_ledger::{staking_ledger_store::StakingLedgerStore, StakingLedger},
    state::{
        ledger::{store::LedgerStore, Ledger},
        Canonicity,
    },
};
use mina_serialization_types::{
    signatures::SignatureJson, staged_ledger_diff::{UserCommand, SignedCommand}, v1::UserCommandWithStatusV1,
};
use rocksdb::{ColumnFamilyDescriptor, DBIterator, DB};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

/// T-{Height}-{Timestamp}-{Hash} -> Transaction
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

    /// Returns the hash of the transaction
    pub fn hash(&self) -> &str {
        &self.2
    }
}

#[derive(Debug)]
pub struct IndexerStore {
    pub db_path: PathBuf,
    pub database: DB,
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
        let tx = ColumnFamilyDescriptor::new("tx", cf_opts.clone());
        let staking_ledgers = ColumnFamilyDescriptor::new("staking-ledgers", cf_opts);

        let mut database_opts = rocksdb::Options::default();
        database_opts.create_missing_column_families(true);
        database_opts.create_if_missing(true);
        let database = rocksdb::DBWithThreadMode::open_cf_descriptors(
            &database_opts,
            path,
            vec![blocks, ledgers, canonicity, tx, staking_ledgers],
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
                let hash = signed_command::SignedCommand(cmd)
                    .hash_signed_command()
                    .unwrap();
                let key = TransactionKey::new(height, timestamp, hash).bytes();
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
            .cf_handle("staking-ledgers")
            .expect("column family exists");

        let key = format!("epoch:{}", epoch);
        let value = bcs::to_bytes(ledger)?;

        self.database.put_cf(&cf_handle, key.as_bytes(), value)?;
        Ok(())
    }

    fn get_epoch(&self, epoch_number: u32) -> anyhow::Result<Option<StakingLedger>> {
        let mut ledger = None;
        let key_str = format!("epoch:{}", epoch_number);
        let key = key_str.as_bytes();
        let cf_handle = self
            .database
            .cf_handle("staking-ledgers")
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
