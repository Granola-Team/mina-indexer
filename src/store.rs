use std::path::{PathBuf, Path};

use rocksdb::{MultiThreaded, DBWithThreadMode};

use crate::block::store::BlockStore;

#[derive(Debug)]
pub struct IndexerStore {
    db_path: PathBuf,
    database: DBWithThreadMode<MultiThreaded>,
}

impl IndexerStore {
    pub fn new_read_only(path: &Path, secondary: &Path) -> anyhow::Result<Self> {
        let database_opts = rocksdb::Options::default();
        let database =
            rocksdb::DBWithThreadMode::open_as_secondary(&database_opts, path, secondary)?;
        Ok(Self {
            db_path: PathBuf::from(path),
            database,
        })
    }
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        let mut database_opts = rocksdb::Options::default();
        database_opts.create_if_missing(true);
        let database = rocksdb::DBWithThreadMode::open(&database_opts, path)?;
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
    fn add_block(&self, block: &crate::block::precomputed::PrecomputedBlock) -> anyhow::Result<()> {
        let key = block.state_hash.as_bytes();
        let value = bcs::to_bytes(&block)?;
        self.database.put(key, value)?;
        Ok(())
    }

    fn get_block(&self, state_hash: &crate::block::BlockHash) -> anyhow::Result<Option<crate::block::precomputed::PrecomputedBlock>> {
        self.database.try_catch_up_with_primary().ok();
        let key = state_hash.0.as_bytes();
        if let Some(bytes) = self.database.get_pinned(key)?.map(|bytes| bytes.to_vec()) {
            let precomputed_block = bcs::from_bytes(&bytes)?;
            return Ok(Some(precomputed_block));
        }
        Ok(None)
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

