use super::precomputed::PcbVersion;
use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    event::db::DbEvent,
    ledger::public_key::PublicKey,
};

pub trait BlockStore {
    /// Add block to the store
    fn add_block(&self, block: &PrecomputedBlock) -> anyhow::Result<Option<DbEvent>>;

    /// Get block from the store
    fn get_block(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PrecomputedBlock>>;

    /// Set best block state hash
    fn set_best_block(&self, state_hash: &BlockHash) -> anyhow::Result<()>;

    /// Get best block from the store
    fn get_best_block(&self) -> anyhow::Result<Option<PrecomputedBlock>>;

    /// Get block hash from the store
    fn get_best_block_hash(&self) -> anyhow::Result<Option<BlockHash>>;

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

    /// Get number of blocks for the given public key
    fn get_num_blocks_at_public_key(&self, pk: &PublicKey) -> anyhow::Result<u32>;

    /// Add block to the given public key's collection
    fn add_block_at_public_key(&self, pk: &PublicKey, state_hash: &BlockHash)
        -> anyhow::Result<()>;

    /// Get blocks for the given public key
    fn get_blocks_at_public_key(&self, pk: &PublicKey) -> anyhow::Result<Vec<PrecomputedBlock>>;

    /// Get children of a block
    fn get_block_children(&self, state_hash: &BlockHash) -> anyhow::Result<Vec<PrecomputedBlock>>;

    /// TODO
    fn set_block_version(&self, state_hash: &BlockHash, version: PcbVersion) -> anyhow::Result<()>;

    /// TODO
    fn get_block_version(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PcbVersion>>;
}
