use std::{path::{Path, PathBuf}, fmt::Display};

use rocksdb::{DBWithThreadMode, MultiThreaded};
use thiserror::Error;

use super::precomputed::PrecomputedBlock;

#[derive(Debug, Clone)]
pub struct BlockStore(pub PathBuf);

impl r2d2::ManageConnection for BlockStore {
    type Connection = BlockStoreConn;

    type Error = BlockStoreError;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        BlockStoreConn::new(&self.0)
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        conn.test_conn()?;
        Ok(())
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}

#[derive(Debug)]
pub struct BlockStoreConn {
    db_path: PathBuf,
    database: DBWithThreadMode<MultiThreaded>,
}

impl BlockStoreConn {
    pub fn new(path: &Path) -> BlockStoreResult<Self> {
        let database_opts = rocksdb::Options::default();
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

    pub fn test_conn(&mut self) -> BlockStoreResult<()> {
        self.database.put("test", "value")?;
        self.database.delete("test")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Error)]
pub enum BlockStoreError {
    DBError(rocksdb::Error)
}
type BlockStoreResult<T> = std::result::Result<T, BlockStoreError>;

impl Display for BlockStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockStoreError::DBError(err) => write!(f, "BlockStoreError: {}", err),
        }
    }
}

impl From<rocksdb::Error> for BlockStoreError {
    fn from(value: rocksdb::Error) -> Self {
        Self::DBError(value)
    }
}