use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    command::internal::InternalCommandWithData,
    ledger::public_key::PublicKey,
};
use speedb::{DBIterator, Direction, IteratorMode, WriteBatch};
use std::path::PathBuf;

/// Store for internal commands
pub trait InternalCommandStore {
    /// Index internal commands for the given block on:
    /// public keys and state hashes
    fn add_internal_commands_batch(
        &self,
        block: &PrecomputedBlock,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()>;

    /// Set the block's `index`-th internal command
    fn set_block_internal_command(
        &self,
        block: &PrecomputedBlock,
        index: u32,
        internal_command: &InternalCommandWithData,
    ) -> anyhow::Result<()>;

    /// Set pk's internal command
    fn set_pk_internal_command(
        &self,
        pk: &PublicKey,
        internal_command: &InternalCommandWithData,
    ) -> anyhow::Result<()>;

    /// Get indexed internal commands from the given block
    fn get_internal_commands(
        &self,
        state_hash: BlockHash,
    ) -> anyhow::Result<Vec<InternalCommandWithData>>;

    /// Get indexed internal command from block
    fn get_block_internal_command(
        &self,
        state_hash: &BlockHash,
        index: u32,
    ) -> anyhow::Result<Option<InternalCommandWithData>>;

    /// Get indexed internal command for the given public key
    fn get_pk_internal_command(
        &self,
        pk: &PublicKey,
        index: u32,
    ) -> anyhow::Result<Option<InternalCommandWithData>>;

    /// Get internal commands for the given public key
    fn get_internal_commands_public_key(
        &self,
        pk: &PublicKey,
        offset: usize,
        limit: usize,
    ) -> anyhow::Result<Vec<InternalCommandWithData>>;

    /// Get number of blocks that the public key has internal commands for
    fn get_pk_num_internal_commands(&self, pk: &PublicKey) -> anyhow::Result<Option<u32>>;

    /// Write the account's internal commands to a CSV file
    fn write_internal_commands_csv(
        &self,
        pk: PublicKey,
        path: Option<PathBuf>,
    ) -> anyhow::Result<PathBuf>;

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
    fn increment_internal_commands_epoch_count(&self, epoch: u32) -> anyhow::Result<()>;

    /// Get internal commands per epoch count
    fn get_internal_commands_epoch_count(&self, epoch: Option<u32>) -> anyhow::Result<u32>;

    /// Increment internal commands total count
    fn increment_internal_commands_total_count(&self) -> anyhow::Result<()>;

    /// Get internal commands total count
    fn get_internal_commands_total_count(&self) -> anyhow::Result<u32>;

    /// Increment internal commands per epoch per account count
    fn increment_internal_commands_pk_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: u32,
    ) -> anyhow::Result<()>;

    /// Get internal commands per epoch per account count
    fn get_internal_commands_pk_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: Option<u32>,
    ) -> anyhow::Result<u32>;

    /// Increment internal commands per account total
    fn increment_internal_commands_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<()>;

    /// Get internal commands per account total
    fn get_internal_commands_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<u32>;

    /// Set internal command count for a block
    fn set_block_internal_commands_count_batch(
        &self,
        state_hash: &BlockHash,
        count: u32,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()>;

    /// Get num internal commands in block
    fn get_block_internal_commands_count(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<u32>>;

    /// Increment internal commands counts given `internal_command` in `epoch`
    fn increment_internal_commands_counts(
        &self,
        internal_command: &InternalCommandWithData,
        epoch: u32,
    ) -> anyhow::Result<()>;
}
