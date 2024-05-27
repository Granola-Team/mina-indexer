use super::precomputed::PcbVersion;
use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    event::db::DbEvent,
    ledger::{diff::account::PaymentDiff, public_key::PublicKey},
};
use std::collections::HashSet;

pub trait BlockStore {
    /// Add block to the store
    fn add_block(&self, block: &PrecomputedBlock) -> anyhow::Result<Option<DbEvent>>;

    /// Get block from the store
    fn get_block(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PrecomputedBlock>>;

    /// Set best block state hash
    fn set_best_block(&self, state_hash: &BlockHash) -> anyhow::Result<()>;

    /// Get best block from the store
    fn get_best_block(&self) -> anyhow::Result<Option<PrecomputedBlock>>;

    /// Get best block hash from the store
    fn get_best_block_hash(&self) -> anyhow::Result<Option<BlockHash>>;

    /// Set a block's previous state hash
    fn set_block_parent_hash(
        &self,
        state_hash: &BlockHash,
        previous_state_hash: &BlockHash,
    ) -> anyhow::Result<()>;

    /// Get a block's parent hash
    fn get_block_parent_hash(&self, state_hash: &BlockHash) -> anyhow::Result<Option<BlockHash>>;

    /// Set a block's blockchain length
    fn set_blockchain_length(
        &self,
        state_hash: &BlockHash,
        blockchain_length: u32,
    ) -> anyhow::Result<()>;

    /// Get a block's blockchain length
    fn get_blockchain_length(&self, state_hash: &BlockHash) -> anyhow::Result<Option<u32>>;

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

    /// Set block height <-> global slot
    fn set_height_global_slot(&self, blockchain_length: u32, slot: u32) -> anyhow::Result<()>;

    /// Get the global slot since genesis corresponding to the given block
    /// height
    fn get_globl_slot_from_height(&self, blockchain_length: u32) -> anyhow::Result<Option<u32>>;

    /// Get the block height corresponding to the global slot since genesis
    fn get_height_from_global_slot(
        &self,
        global_slot_since_genesis: u32,
    ) -> anyhow::Result<Option<u32>>;

    /// Get number of blocks for the given public key
    fn get_num_blocks_at_public_key(&self, pk: &PublicKey) -> anyhow::Result<u32>;

    /// Add block to the given public key's collection
    fn add_block_at_public_key(&self, pk: &PublicKey, state_hash: &BlockHash)
        -> anyhow::Result<()>;

    /// Get blocks for the given public key
    fn get_blocks_at_public_key(&self, pk: &PublicKey) -> anyhow::Result<Vec<PrecomputedBlock>>;

    /// Get children of a block
    fn get_block_children(&self, state_hash: &BlockHash) -> anyhow::Result<Vec<PrecomputedBlock>>;

    /// Set block version
    fn set_block_version(&self, state_hash: &BlockHash, version: PcbVersion) -> anyhow::Result<()>;

    /// Get the block's version
    fn get_block_version(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PcbVersion>>;

    /// Update pk's balance-sorted account balance
    fn update_account_balance(&self, pk: &PublicKey, balance: Option<u64>) -> anyhow::Result<()>;

    /// Generate account balance updates when the best tip changes.
    /// Return with set of coinbase receivers.
    fn common_ancestor_account_balance_updates(
        &self,
        old_best_tip: &BlockHash,
        new_best_tip: &BlockHash,
    ) -> anyhow::Result<(Vec<PaymentDiff>, HashSet<PublicKey>)>;

    /// Set the balance updates for a block
    fn set_block_balance_updates(
        &self,
        state_hash: &BlockHash,
        coinbase_receiver: PublicKey,
        balance_updates: Vec<PaymentDiff>,
    ) -> anyhow::Result<()>;

    /// Get a block's balance updates
    fn get_block_balance_updates(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<(PublicKey, Vec<PaymentDiff>)>>;

    /// Updates stored account balances
    fn update_account_balances(
        &self,
        state_hash: &BlockHash,
        updates: Vec<PaymentDiff>,
        coinbase_receivers: HashSet<PublicKey>,
    ) -> anyhow::Result<()>;

    /// Get the epoch count of the best block
    fn get_current_epoch(&self) -> anyhow::Result<u32>;

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

    /// Get the total block production count for `epoch`
    fn get_block_production_epoch_count(&self, epoch: u32) -> anyhow::Result<u32>;

    /// Get the total block production count
    fn get_block_production_total_count(&self) -> anyhow::Result<u32>;

    fn get_coinbase_receiver(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PublicKey>>;

    fn set_coinbase_receiver(
        &self,
        state_hash: &BlockHash,
        coinbase_receiver: &PublicKey,
    ) -> anyhow::Result<()>;
}
