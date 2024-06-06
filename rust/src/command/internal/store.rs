use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    command::internal::InternalCommandWithData,
    ledger::public_key::PublicKey,
};
use speedb::DBIterator;

/// Store for internal commands
pub trait InternalCommandStore {
    /// Index internal commands for the given block on:
    /// public keys and state hashes
    fn add_internal_commands(&self, block: &PrecomputedBlock) -> anyhow::Result<()>;

    /// Get indexed internal commands from the given block
    fn get_internal_commands(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Vec<InternalCommandWithData>>;

    /// Get indexed internal commands for the given public key
    fn get_internal_commands_public_key(
        &self,
        pk: &PublicKey,
    ) -> anyhow::Result<Vec<InternalCommandWithData>>;

    /// Get number of blocks that the public key has internal commands for
    fn get_pk_num_internal_commands(&self, pk: &str) -> anyhow::Result<Option<u32>>;

    /// Get internal commands interator (by global slot) with given mode
    fn internal_commands_global_slot_interator(&self, mode: speedb::IteratorMode)
        -> DBIterator<'_>;

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
    fn set_block_internal_commands_count(
        &self,
        state_hash: &BlockHash,
        count: u32,
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
