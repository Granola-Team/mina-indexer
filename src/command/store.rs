use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    command::signed::SignedCommand,
    ledger::public_key::PublicKey,
};

///
pub trait CommandStore {
    /// Add commands (transactions) from the given block indexed on:
    /// public keys, transaction hash, and state hashes
    fn add_commands(&self, block: &PrecomputedBlock) -> anyhow::Result<()>;

    /// Get commands from the given block
    fn get_commands_in_block(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<Vec<SignedCommand>>>;

    /// Get a command by its hash
    fn get_command_by_hash(&self, command_hash: &str) -> anyhow::Result<Option<SignedCommand>>;

    /// Get commands involving the public key as a sender or receiver
    fn get_commands_public_key(&self, pk: &PublicKey)
        -> anyhow::Result<Option<Vec<SignedCommand>>>;
}
