use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    event::db::DbEvent,
};

pub trait BlockStore {
    /// Add block to the store
    fn add_block(&self, block: &PrecomputedBlock) -> anyhow::Result<DbEvent>;

    /// Get block from the store
    fn get_block(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PrecomputedBlock>>;
}
