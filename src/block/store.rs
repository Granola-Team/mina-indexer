use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use rocksdb::{DBWithThreadMode, MultiThreaded};
use thiserror::Error;

use super::precomputed::PrecomputedBlock;

#[derive(Debug)]
pub struct BlockStoreConn {
    db_path: PathBuf,
    database: DBWithThreadMode<MultiThreaded>,
}

impl BlockStoreConn {
    pub fn new_read_only(path: &Path, secondary: &Path) -> BlockStoreResult<Self> {
        let database_opts = rocksdb::Options::default();
        let database =
            rocksdb::DBWithThreadMode::open_as_secondary(&database_opts, path, secondary)?;
        Ok(Self {
            db_path: PathBuf::from(path),
            database,
        })
    }
    pub fn new(path: &Path) -> BlockStoreResult<Self> {
        let mut database_opts = rocksdb::Options::default();
        database_opts.create_if_missing(true);
        let database = rocksdb::DBWithThreadMode::open(&database_opts, path)?;
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
        self.database.try_catch_up_with_primary().ok();
        let key = state_hash.as_bytes();
        if let Some(bytes) = self.database.get_pinned(key)?.map(|bytes| bytes.to_vec()) {
            let precomputed_block = bcs::from_bytes(&bytes)?;
            return Ok(Some(precomputed_block));
        }
        Ok(None)
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn test_conn(&mut self) -> BlockStoreResult<()> {
        self.database.put("test", "value")?;
        self.database.delete("test")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Error)]
pub enum BlockStoreError {
    DBError(rocksdb::Error),
}
type BlockStoreResult<T> = std::result::Result<T, BlockStoreError>;

impl Display for BlockStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockStoreError::DBError(err) => {
                write!(f, "{}", format_args!("BlockStoreError: {err}"))
            }
        }
    }
}

impl From<rocksdb::Error> for BlockStoreError {
    fn from(value: rocksdb::Error) -> Self {
        Self::DBError(value)
    }
}
