use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    command::{signed::SignedCommandWithStateHash, UserCommandWithStatus},
    ledger::public_key::PublicKey,
};

pub trait CommandStore {
    /// Add commands (transactions) from the given block indexed on:
    /// public keys, transaction hash, and state hashes
    fn add_commands(&self, block: &PrecomputedBlock) -> anyhow::Result<()>;

    /// Get commands from the given block
    fn get_commands_in_block(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<Vec<UserCommandWithStatus>>>;

    /// Get a command by its hash
    fn get_command_by_hash(
        &self,
        command_hash: &str,
    ) -> anyhow::Result<Option<SignedCommandWithStateHash>>;

    /// Get commands involving the public key as a sender or receiver
    fn get_commands_for_public_key(
        &self,
        pk: &PublicKey,
    ) -> anyhow::Result<Option<Vec<SignedCommandWithStateHash>>>;

    /// Get commands for the public key with number and/or state hash bounds
    fn get_commands_with_bounds(
        &self,
        pk: &PublicKey,
        start_state_hash: &BlockHash,
        end_state_hash: &BlockHash,
    ) -> anyhow::Result<Option<Vec<SignedCommandWithStateHash>>>;

    /// Get number of commands for public key `pk`
    fn get_pk_num_commands(&self, pk: &str) -> anyhow::Result<Option<u32>>;
}
