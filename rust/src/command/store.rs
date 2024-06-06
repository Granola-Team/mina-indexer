use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    command::{signed::SignedCommandWithData, UserCommandWithStatus},
    ledger::public_key::PublicKey,
};

// TODO add iterators
// use speedb::DBIterator;

/// Store for user commands
pub trait UserCommandStore {
    /// Index user commands (transactions) from the given block on:
    /// public keys, transaction hash, and state hashes
    fn add_user_commands(&self, block: &PrecomputedBlock) -> anyhow::Result<()>;

    /// Get indexed user commands from the given block
    fn get_user_commands_in_block(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Vec<UserCommandWithStatus>>;

    /// Get indexed user command by its hash
    fn get_user_command_by_hash(
        &self,
        command_hash: &str,
    ) -> anyhow::Result<Option<SignedCommandWithData>>;

    /// Get indexed user commands involving the public key as a sender or
    /// receiver
    fn get_user_commands_for_public_key(
        &self,
        pk: &PublicKey,
    ) -> anyhow::Result<Vec<SignedCommandWithData>>;

    /// Get user commands for the public key with number and/or state hash
    /// bounds
    fn get_user_commands_with_bounds(
        &self,
        pk: &PublicKey,
        start_state_hash: &BlockHash,
        end_state_hash: &BlockHash,
    ) -> anyhow::Result<Vec<SignedCommandWithData>>;

    fn get_pk_num_user_commands(&self, pk: &str) -> anyhow::Result<Option<u32>>;

    /// Increment user commands per epoch count
    fn increment_user_commands_epoch_count(&self, epoch: u32) -> anyhow::Result<()>;

    /// Get user commands per epoch count
    fn get_user_commands_epoch_count(&self, epoch: Option<u32>) -> anyhow::Result<u32>;

    /// Increment user commands total count
    fn increment_user_commands_total_count(&self) -> anyhow::Result<()>;

    /// Get user commands total count
    fn get_user_commands_total_count(&self) -> anyhow::Result<u32>;

    /// Increment user commands per epoch per account count
    fn increment_user_commands_pk_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: u32,
    ) -> anyhow::Result<()>;

    /// Get user commands per epoch per account count
    fn get_user_commands_pk_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: Option<u32>,
    ) -> anyhow::Result<u32>;

    /// Increment user commands per account total
    fn increment_user_commands_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<()>;

    /// Get user commands per account total
    fn get_user_commands_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<u32>;

    /// Increment user commands per block
    fn set_block_user_commands_count(
        &self,
        state_hash: &BlockHash,
        count: u32,
    ) -> anyhow::Result<()>;

    /// Get user commands per block
    fn get_block_user_commands_count(&self, state_hash: &BlockHash) -> anyhow::Result<Option<u32>>;

    /// Increment user commands counts given `command` in `epoch`
    fn increment_user_commands_counts(
        &self,
        command: &UserCommandWithStatus,
        epoch: u32,
    ) -> anyhow::Result<()>;
}
