use crate::{
    base::state_hash::StateHash,
    block::store::DbBlockUpdate,
    canonicity::{Canonicity, CanonicityUpdate},
};

pub trait CanonicityStore {
    /// Add the canonical block's height, global slot, and state hash
    fn add_canonical_block(
        &self,
        height: u32,
        global_slot: u32,
        state_hash: &StateHash,
        genesis_state_hash: &StateHash,
        genesis_prev_state_hash: Option<&StateHash>,
    ) -> anyhow::Result<()>;

    /// Update block canonicities
    fn update_block_canonicities(&self, blocks: &DbBlockUpdate) -> anyhow::Result<()>;

    fn update_canonicity(&self, updates: CanonicityUpdate) -> anyhow::Result<()>;

    /// Get the state hash of the canonical block at the given height
    fn get_canonical_hash_at_height(&self, height: u32) -> anyhow::Result<Option<StateHash>>;

    /// Get the state hash of the canonical block at the given global slot
    fn get_canonical_hash_at_slot(&self, global_slot: u32) -> anyhow::Result<Option<StateHash>>;

    /// Get block canonicity
    fn get_block_canonicity(&self, state_hash: &StateHash) -> anyhow::Result<Option<Canonicity>>;

    /// Get the list of all known genesis state hashes
    fn get_known_genesis_state_hashes(&self) -> anyhow::Result<Vec<StateHash>>;

    /// Get the list of all known genesis prev state hashes
    fn get_known_genesis_prev_state_hashes(&self) -> anyhow::Result<Vec<StateHash>>;
}
