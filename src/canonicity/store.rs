use crate::{block::BlockHash, canonicity::Canonicity};

pub trait CanonicityStore {
    /// Get the length of the canonical chain
    fn get_max_canonical_blockchain_length(&self) -> anyhow::Result<Option<u32>>;

    /// Set the length of the canonical chain
    fn set_max_canonical_blockchain_length(&self, height: u32) -> anyhow::Result<()>;

    /// Add the canonical block's height and state hash
    fn add_canonical_block(&self, height: u32, state_hash: &BlockHash) -> anyhow::Result<()>;

    /// Get the state hash of the canonical block at the given height
    fn get_canonical_hash_at_height(&self, height: u32) -> anyhow::Result<Option<BlockHash>>;

    /// Get block canonicity
    fn get_block_canonicity(
        &self,
        state_hash: &BlockHash,
        best_tip: &BlockHash,
    ) -> anyhow::Result<Option<Canonicity>>;
}
