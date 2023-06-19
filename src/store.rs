use std::path::{Path, PathBuf};

use rocksdb::{ColumnFamilyDescriptor, DBWithThreadMode, MultiThreaded};

use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore, BlockHash},
    state::ledger::{store::LedgerStore, Ledger},
};

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
        let ledgers = ColumnFamilyDescriptor::new("ledgers", cf_opts);

        let mut database_opts = rocksdb::Options::default();
        database_opts.create_missing_column_families(true);
        database_opts.create_if_missing(true);
        let database = rocksdb::DBWithThreadMode::open_cf_descriptors(
            &database_opts,
            path,
            vec![blocks, ledgers],
        )?;
        Ok(Self {
            db_path: PathBuf::from(path),
            database,
        })
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
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
        let cf_handle = self
            .database
            .cf_handle("ledgers")
            .expect("column family exists");
        let mut ledger = None;
        self.database.try_catch_up_with_primary().ok();
        let key = state_hash.0.as_bytes();
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

    pub fn x(&self) -> String {
        self.database
            .property_value(rocksdb::properties::DBSTATS)
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
