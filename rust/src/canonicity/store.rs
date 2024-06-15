use crate::{
    block::BlockHash,
    canonicity::{Canonicity, CanonicityUpdate},
};

pub trait CanonicityStore {
    /// Add the canonical block's height, global slot, and state hash
    fn add_canonical_block(
        &self,
        height: u32,
        global_slot: u32,
        state_hash: &BlockHash,
        genesis_state_hash: &BlockHash,
        genesis_prev_state_hash: Option<&BlockHash>,
    ) -> anyhow::Result<()>;

    /// Update block canonicities
    fn update_canonicity(&self, updates: CanonicityUpdate) -> anyhow::Result<()>;

    /// Generate canonicity updates when the best tip changes
    fn reorg_canonicity_updates(
        &self,
        old_best_tip: &BlockHash,
        new_best_tip: &BlockHash,
    ) -> anyhow::Result<CanonicityUpdate>;

    /// Get the state hash of the canonical block at the given height
    fn get_canonical_hash_at_height(&self, height: u32) -> anyhow::Result<Option<BlockHash>>;

    /// Get the state hash of the canonical block at the given global slot
    fn get_canonical_hash_at_slot(&self, global_slot: u32) -> anyhow::Result<Option<BlockHash>>;

    /// Get block canonicity
    fn get_block_canonicity(&self, state_hash: &BlockHash) -> anyhow::Result<Option<Canonicity>>;

    /// Get the list of all known genesis state hashes
    fn get_known_genesis_state_hashes(&self) -> anyhow::Result<Vec<BlockHash>>;

    /// Get the list of all known genesis prev state hashes
    fn get_known_genesis_prev_state_hashes(&self) -> anyhow::Result<Vec<BlockHash>>;
}
