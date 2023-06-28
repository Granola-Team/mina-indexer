use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore, BlockHash},
    state::{
        ledger::{store::LedgerStore, Ledger},
        Canonicity,
    },
};
use mina_serialization_types::{staged_ledger_diff::UserCommand, v1::UserCommandWithStatusV1};
use rocksdb::{ColumnFamilyDescriptor, DBWithThreadMode, MultiThreaded};
use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
};

/// Storage Key
pub struct Key<T>(PhantomData<T>);

/// T-{Height}-{Signature} -> Transaction
/// We use the signature as key until we have a better way to identify transactions (e.g. hash)
/// The height is padded to 12 digits for sequential iteration
pub struct Transaction;

impl Key<Transaction> {
    /// Creates a new key for a transaction as string
    pub fn str<S>(h: u32, s: S) -> String
    where
        S: Into<String>,
    {
        format!("T-{:012}-{}", h, s.into())
    }

    /// Creates a new key for a transaction as bytes
    pub fn bytes<S>(h: u32, s: S) -> Vec<u8>
    where
        S: Into<String>,
    {
        Self::str(h, s).into_bytes()
    }
}

#[derive(Debug)]
pub struct IndexerStore {
    db_path: PathBuf,
    database: DBWithThreadMode<MultiThreaded>,
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

    pub fn put_tx(&self, height: u32, tx: UserCommandWithStatusV1) -> anyhow::Result<()> {
        let cf_handle = self.database.cf_handle("tx").expect("column family exists");

        match tx.clone().inner().data.inner().inner() {
            UserCommand::SignedCommand(cmd) => {
                let sig = serde_json::to_string(&cmd.inner().inner().signature)?;

                let key = Key::<Transaction>::bytes(height, sig);
                let value = bcs::to_bytes(&tx)?;

                self.database.put_cf(&cf_handle, key, value)?;

                Ok(())
            }
        }
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
