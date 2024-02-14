use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    event::db::DbEvent,
};

pub trait BlockStore {
    /// Add block to the store
    fn add_block(&self, block: &PrecomputedBlock) -> anyhow::Result<DbEvent>;

    /// Get block from the store
    fn get_block(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PrecomputedBlock>>;

    /// Set best block state hash
    fn set_best_block(&self, state_hash: &BlockHash) -> anyhow::Result<()>;

    /// Get best block from the store
    fn get_best_block(&self) -> anyhow::Result<Option<PrecomputedBlock>>;

    /// Get number of blocks at the given blockchain length
    fn get_num_blocks_at_height(&self, blockchain_length: u32) -> anyhow::Result<u32>;

    /// Get all blocks at the given blockchain length
    fn get_blocks_at_height(&self, blockchain_length: u32)
        -> anyhow::Result<Vec<PrecomputedBlock>>;

    /// Add a block at the given blockchain length
    fn add_block_at_height(
        &self,
        state_hash: &BlockHash,
        blockchain_length: u32,
    ) -> anyhow::Result<()>;

    /// Get number of blocks at the given global slot since genesis
    fn get_num_blocks_at_slot(&self, slot: u32) -> anyhow::Result<u32>;

    /// Get all blocks at the given global slot since genesis
    fn get_blocks_at_slot(&self, slot: u32) -> anyhow::Result<Vec<PrecomputedBlock>>;

    /// Add a block at the given global slot since genesis
    fn add_block_at_slot(&self, state_hash: &BlockHash, slot: u32) -> anyhow::Result<()>;
}
