use super::{precomputed::PcbVersion, BlockComparison};
use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    event::db::DbEvent,
    ledger::public_key::PublicKey,
};
use speedb::{DBIterator, IteratorMode};

pub trait BlockStore {
    /// Add block to the store
    fn add_block(&self, block: &PrecomputedBlock) -> anyhow::Result<Option<DbEvent>>;

    /// Get block from the store
    fn get_block(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PrecomputedBlock>>;

    //////////////////////////
    // Best block functions //
    //////////////////////////

    /// Index best block state hash
    fn set_best_block(&self, state_hash: &BlockHash) -> anyhow::Result<()>;

    /// Get best block from the store
    fn get_best_block(&self) -> anyhow::Result<Option<PrecomputedBlock>>;

    /// Get best block epoch count without deserializing the PCB
    fn get_current_epoch(&self) -> anyhow::Result<u32>;

    /// Get best block state hash without deserializing the PCB
    fn get_best_block_hash(&self) -> anyhow::Result<Option<BlockHash>>;

    /// Get best block height without deserializing the PCB
    fn get_best_block_height(&self) -> anyhow::Result<Option<u32>>;

    /// Get best block global slot without deserializing the PCB
    fn get_best_block_global_slot(&self) -> anyhow::Result<Option<u32>>;

    /// Get best block genesis state hash without deserializing the PCB
    fn get_best_block_genesis_hash(&self) -> anyhow::Result<Option<BlockHash>>;

    /////////////////////////////
    // General block functions //
    /////////////////////////////

    /// Index the block's previous state hash
    fn set_block_parent_hash(
        &self,
        state_hash: &BlockHash,
        previous_state_hash: &BlockHash,
    ) -> anyhow::Result<()>;

    /// Get a block's parent hash
    fn get_block_parent_hash(&self, state_hash: &BlockHash) -> anyhow::Result<Option<BlockHash>>;

    /// Index the block's blockchain length
    fn set_block_height(
        &self,
        state_hash: &BlockHash,
        blockchain_length: u32,
    ) -> anyhow::Result<()>;

    /// Get a block's blockchain length without deserializing the PCB
    fn get_block_height(&self, state_hash: &BlockHash) -> anyhow::Result<Option<u32>>;

    /// Index the block's global slot
    fn set_block_global_slot(&self, state_hash: &BlockHash, global_slot: u32)
        -> anyhow::Result<()>;

    /// Get a block's global slot without deserializing the PCB
    fn get_block_global_slot(&self, state_hash: &BlockHash) -> anyhow::Result<Option<u32>>;

    /// Index the block's epoch count
    fn set_block_epoch(&self, state_hash: &BlockHash, epoch: u32) -> anyhow::Result<()>;

    /// Get the block's epoch count without deserializing the PCB
    fn get_block_epoch(&self, state_hash: &BlockHash) -> anyhow::Result<Option<u32>>;

    /// Index the block's genesis state hash
    fn set_block_genesis_state_hash(
        &self,
        state_hash: &BlockHash,
        genesis_state_hash: &BlockHash,
    ) -> anyhow::Result<()>;

    /// Get the given block's genesis state hash without deserializing the PCB
    fn get_block_genesis_state_hash(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<BlockHash>>;

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

    /// Include in one another's collection
    fn set_block_height_global_slot_pair(
        &self,
        blockchain_length: u32,
        global_slot: u32,
    ) -> anyhow::Result<()>;

    /// Get the global slots corresponding to the given block height
    fn get_global_slots_from_height(
        &self,
        blockchain_length: u32,
    ) -> anyhow::Result<Option<Vec<u32>>>;

    /// Get the block heights corresponding to the global slot since genesis
    fn get_block_heights_from_global_slot(
        &self,
        global_slot: u32,
    ) -> anyhow::Result<Option<Vec<u32>>>;

    /// Get number of blocks for the given public key
    fn get_num_blocks_at_public_key(&self, pk: &PublicKey) -> anyhow::Result<u32>;

    /// Add block to the given public key's collection
    fn add_block_at_public_key(&self, pk: &PublicKey, state_hash: &BlockHash)
        -> anyhow::Result<()>;

    /// Get blocks for the given public key
    fn get_blocks_at_public_key(&self, pk: &PublicKey) -> anyhow::Result<Vec<PrecomputedBlock>>;

    /// Get children of a block
    fn get_block_children(&self, state_hash: &BlockHash) -> anyhow::Result<Vec<PrecomputedBlock>>;

    /// Index block version
    fn set_block_version(&self, state_hash: &BlockHash, version: PcbVersion) -> anyhow::Result<()>;

    /// Get the block's version
    fn get_block_version(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PcbVersion>>;

    /// Get the indexed coinbase receiver for the given block
    fn get_coinbase_receiver(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PublicKey>>;

    /// Index the coinbase receiver for the given block
    fn set_coinbase_receiver(
        &self,
        state_hash: &BlockHash,
        coinbase_receiver: &PublicKey,
    ) -> anyhow::Result<()>;

    /// Index the block's minimimal info needed for comparison
    fn set_block_comparison(
        &self,
        state_hash: &BlockHash,
        comparison: &BlockComparison,
    ) -> anyhow::Result<()>;

    /// Get the info needed for block comparison without deserializing the PCB
    fn get_block_comparison(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<BlockComparison>>;

    /// Compare blocks without deserializing the PCBs
    fn block_cmp(
        &self,
        block: &BlockHash,
        other: &BlockHash,
    ) -> anyhow::Result<Option<std::cmp::Ordering>>;

    ///////////////
    // Iterators //
    ///////////////

    /// Iterator for blocks via height
    /// ```
    /// key: {block_height}{state_hash}
    /// val: b""
    /// ```
    /// Use [block_state_hash_from_key] to extract state hash
    fn blocks_height_iterator<'a>(&'a self, mode: IteratorMode) -> DBIterator<'a>;

    /// Iterator for blocks via global slot
    /// ```
    /// key: {global_slot}{state_hash}
    /// val: b""
    /// ```
    /// Use [block_state_hash_from_key] to extract state hash
    fn blocks_global_slot_iterator<'a>(&'a self, mode: IteratorMode) -> DBIterator<'a>;

    //////////////////
    // Block counts //
    //////////////////

    /// Increment the epoch & pk block production counts
    fn increment_block_production_count(&self, block: &PrecomputedBlock) -> anyhow::Result<()>;

    /// Get the block production count for `pk` in `epoch`
    /// (default: current epoch)
    fn get_block_production_pk_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: Option<u32>,
    ) -> anyhow::Result<u32>;

    /// Get the total block production count for `pk`
    fn get_block_production_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<u32>;

    /// Get the total block production count for `epoch` (default: current
    /// epoch)
    fn get_block_production_epoch_count(&self, epoch: Option<u32>) -> anyhow::Result<u32>;

    /// Get the total block production count
    fn get_block_production_total_count(&self) -> anyhow::Result<u32>;

    ///////////////////////////////
    // Dump block store contents //
    ///////////////////////////////

    /// Dump blocks via height to `path`
    fn dump_blocks_via_height(&self, path: &std::path::Path) -> anyhow::Result<()>;

    /// Blocks via height
    fn blocks_via_height(&self, mode: IteratorMode) -> anyhow::Result<Vec<PrecomputedBlock>>;

    /// Dump blocks via global slot to `path`
    fn dump_blocks_via_global_slot(&self, path: &std::path::Path) -> anyhow::Result<()>;

    /// Blocks via global_slot
    fn blocks_via_global_slot(&self, mode: IteratorMode) -> anyhow::Result<Vec<PrecomputedBlock>>;
}
