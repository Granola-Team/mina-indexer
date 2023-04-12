use std::path::{Path, PathBuf};

use super::precomputed::PrecomputedBlock;

#[derive(Debug)]
pub struct BlockStore {
    db_path: PathBuf,
    database: rocksdb::DB,
}

impl BlockStore {
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        // let database_opts = rocksdb::Options::default();
        let database = rocksdb::DB::open_default(path)?;
        Ok(Self {
            db_path: PathBuf::from(path),
            database,
        })
    }

    pub fn add_block(&self, block: &PrecomputedBlock) -> anyhow::Result<()> {
        let key = block.state_hash.as_bytes();
        let value = bcs::to_bytes(&block)?;
        self.database.put(key, value)?;
        Ok(())
    }

    pub fn get_block(&self, state_hash: &str) -> anyhow::Result<Option<PrecomputedBlock>> {
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
