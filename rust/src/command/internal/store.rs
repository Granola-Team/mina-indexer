//! Internal command store trait

use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::{precomputed::PrecomputedBlock, store::DbBlockUpdate},
    command::internal::DbInternalCommandWithData,
    store::Result,
};
use speedb::{DBIterator, Direction, IteratorMode, WriteBatch};
use std::path::PathBuf;

pub trait InternalCommandStore {
    /// Index internal commands for the given block on:
    /// public keys and state hashes
    fn add_internal_commands_batch(
        &self,
        block: &PrecomputedBlock,
        batch: &mut WriteBatch,
    ) -> Result<()>;

    /// Set the block's `index`-th internal command
    fn set_block_internal_command(
        &self,
        block: &PrecomputedBlock,
        index: u32,
        internal_command: &DbInternalCommandWithData,
    ) -> Result<()>;

    /// Set pk's internal command
    fn set_pk_internal_command(
        &self,
        pk: &PublicKey,
        internal_command: &DbInternalCommandWithData,
    ) -> Result<()>;

    /// Get indexed internal commands from the given block
    fn get_internal_commands(
        &self,
        state_hash: &StateHash,
    ) -> Result<Vec<DbInternalCommandWithData>>;

    /// Get indexed internal command from block
    fn get_block_internal_command(
        &self,
        state_hash: &StateHash,
        index: u32,
    ) -> Result<Option<DbInternalCommandWithData>>;

    /// Get indexed internal command for the given public key
    fn get_pk_internal_command(
        &self,
        pk: &PublicKey,
        index: u32,
    ) -> Result<Option<DbInternalCommandWithData>>;

    /// Get internal commands for the given public key
    fn get_internal_commands_public_key(
        &self,
        pk: &PublicKey,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<DbInternalCommandWithData>>;

    /// Get number of blocks that the public key has internal commands for
    fn get_pk_num_internal_commands(&self, pk: &PublicKey) -> Result<Option<u32>>;

    /// Write the account's internal commands to a CSV file
    fn write_internal_commands_csv(&self, pk: PublicKey, path: Option<PathBuf>) -> Result<PathBuf>;

    ///////////////
    // Iterators //
    ///////////////

    /// Internal commands iterator via block height
    fn internal_commands_block_height_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Internal commands iterator via global slot
    fn internal_commands_global_slot_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Account internal commands iterator via block height
    fn internal_commands_pk_block_height_iterator(
        &self,
        pk: PublicKey,
        direction: Direction,
    ) -> DBIterator<'_>;

    /// Account internal commands iterator via global slot
    fn internal_commands_pk_global_slot_iterator(
        &self,
        pk: PublicKey,
        direction: Direction,
    ) -> DBIterator<'_>;

    /////////////////////////////
    // Internal command counts //
    /////////////////////////////

    /// Increment internal commands per epoch count
    fn increment_internal_commands_epoch_count(
        &self,
        epoch: u32,
        genesis_state_hash: &StateHash,
    ) -> Result<()>;

    /// Get internal commands per epoch count
    fn get_internal_commands_epoch_count(
        &self,
        epoch: Option<u32>,
        genesis_state_hash: Option<&StateHash>,
    ) -> Result<u32>;

    /// Increment internal commands total count
    fn increment_internal_commands_total_count(&self, incr: u32) -> Result<()>;

    /// Get internal commands total count
    fn get_internal_commands_total_count(&self) -> Result<u32>;

    /// Increment internal commands per epoch per account count
    fn increment_internal_commands_pk_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: u32,
        genesis_state_hash: &StateHash,
    ) -> Result<()>;

    /// Get internal commands per epoch per account count
    fn get_internal_commands_pk_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: Option<u32>,
        genesis_state_hash: Option<&StateHash>,
    ) -> Result<u32>;

    /// Increment internal commands per account total
    fn increment_internal_commands_pk_total_count(&self, pk: &PublicKey) -> Result<()>;

    /// Get internal commands per account total
    fn get_internal_commands_pk_total_count(&self, pk: &PublicKey) -> Result<u32>;

    /// Set internal command count for a block
    fn set_block_internal_commands_count_batch(
        &self,
        state_hash: &StateHash,
        count: u32,
        batch: &mut WriteBatch,
    ) -> Result<()>;

    /// Get num internal commands in block
    fn get_block_internal_commands_count(&self, state_hash: &StateHash) -> Result<Option<u32>>;

    /// Increment internal commands counts given `internal_command` in `epoch`
    fn increment_internal_commands_counts(
        &self,
        internal_command: &DbInternalCommandWithData,
        epoch: u32,
        genesis_state_hash: &StateHash,
    ) -> Result<()>;

    /// get canonical internal commands count
    fn get_canonical_internal_commands_count(&self) -> Result<u32>;

    /// Increment canonical internal commands count
    fn increment_canonical_internal_commands_count(&self, incr: u32) -> Result<()>;

    /// Decrement canonical internal commands count
    fn decrement_canonical_internal_commands_count(&self, incr: u32) -> Result<()>;

    /// Internal commands from DbBlockUpdate
    fn update_internal_commands(&self, blocks: &DbBlockUpdate) -> Result<()>;
}
