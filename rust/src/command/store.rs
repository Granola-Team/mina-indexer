use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    command::{
        internal::InternalCommandWithData, signed::SignedCommandWithData, UserCommandWithStatus,
    },
    ledger::public_key::PublicKey,
};

/// Store for internal & user commands
pub trait CommandStore {
    /// Index user commands (transactions) from the given block on:
    /// public keys, transaction hash, and state hashes
    fn add_commands(&self, block: &PrecomputedBlock) -> anyhow::Result<()>;

    /// Get indexed user commands from the given block
    fn get_commands_in_block(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Vec<UserCommandWithStatus>>;

    /// Get indexed user command by its hash
    fn get_command_by_hash(
        &self,
        command_hash: &str,
    ) -> anyhow::Result<Option<SignedCommandWithData>>;

    /// Get indexed user commands involving the public key as a sender or
    /// receiver
    fn get_commands_for_public_key(
        &self,
        pk: &PublicKey,
    ) -> anyhow::Result<Vec<SignedCommandWithData>>;

    /// Get user commands for the public key with number and/or state hash
    /// bounds
    fn get_commands_with_bounds(
        &self,
        pk: &PublicKey,
        start_state_hash: &BlockHash,
        end_state_hash: &BlockHash,
    ) -> anyhow::Result<Vec<SignedCommandWithData>>;

    fn get_pk_num_commands(&self, pk: &str) -> anyhow::Result<Option<u32>>;

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

    fn get_pk_num_internal_commands(&self, pk: &str) -> anyhow::Result<Option<u32>>;
}
