use std::path::{Path, PathBuf};

use super::precomputed::PrecomputedBlock;

pub struct BlockStore {
    db_path: PathBuf,
    database: rocksdb::DB,
}

impl BlockStore {
    pub fn new(path: &Path) -> Result<Self, anyhow::Error> {
        let database_opts = rocksdb::Options::default();
        let database = rocksdb::DB::open(&database_opts, path)?;
        Ok(Self {
            db_path: PathBuf::from(path),
            database,
        })
    }

    pub fn add_block(&self, block: &PrecomputedBlock) -> Result<(), anyhow::Error> {
        let key = block.state_hash.as_bytes();
        let value = bcs::to_bytes(&block)?;
        self.database.put(key, value)?;
        Ok(())
    }

    pub fn get_block(&self, state_hash: &str) -> Result<Option<PrecomputedBlock>, anyhow::Error> {
        let key = state_hash.as_bytes();
        if let Some(bytes) = self.database.get(key)? {
            let precomputed_block = bcs::from_bytes(&bytes)?;
            return Ok(Some(precomputed_block));
        }
        Ok(None)
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}
