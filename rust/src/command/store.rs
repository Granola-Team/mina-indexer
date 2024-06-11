use super::signed::TXN_HASH_LEN;
use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    command::{signed::SignedCommandWithData, UserCommandWithStatus},
    ledger::public_key::PublicKey,
    store::from_be_bytes,
};
use anyhow::anyhow;
use speedb::{DBIterator, IteratorMode};

/// Store for user commands
pub trait UserCommandStore {
    /// Index user commands (transactions) from the given block on:
    /// public keys, transaction hash, and state hashes
    fn add_user_commands(&self, block: &PrecomputedBlock) -> anyhow::Result<()>;

    /// Set user commands for the given block
    fn set_block_user_commands(&self, block: &PrecomputedBlock) -> anyhow::Result<()>;

    /// Get indexed user commands from the given block
    fn get_block_user_commands(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<Vec<UserCommandWithStatus>>>;

    /// Get user command by its hash & index
    fn get_user_command(
        &self,
        txn_hash: &str,
        index: u32,
    ) -> anyhow::Result<Option<SignedCommandWithData>>;

    /// Get user command by its hash & containing block
    fn get_user_command_state_hash(
        &self,
        txn_hash: &str,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<SignedCommandWithData>>;

    /// Get indexed user commands involving the public key as a sender or
    /// receiver
    fn get_user_commands_for_public_key(
        &self,
        pk: &PublicKey,
    ) -> anyhow::Result<Option<Vec<SignedCommandWithData>>>;

    /// Get user commands for the public key with number and/or state hash
    /// bounds
    fn get_user_commands_with_bounds(
        &self,
        pk: &PublicKey,
        start_state_hash: &BlockHash,
        end_state_hash: &BlockHash,
    ) -> anyhow::Result<Vec<SignedCommandWithData>>;

    /// Set block containing `txn_hash`
    fn set_user_command_state_hash(
        &self,
        state_hash: BlockHash,
        txn_hash: &str,
    ) -> anyhow::Result<()>;

    /// Get state hashes of blocks containing `txn_hash` in block sorted order
    fn get_user_command_state_hashes(
        &self,
        txn_hash: &str,
    ) -> anyhow::Result<Option<Vec<BlockHash>>>;

    /// Get number of blocks containing `txn_hash`
    fn get_user_commands_num_containing_blocks(
        &self,
        txn_hash: &str,
    ) -> anyhow::Result<Option<u32>>;

    ///////////////
    // Iterators //
    ///////////////

    /// Iterator for user commands via global slot
    fn user_commands_slot_iterator<'a>(&'a self, mode: IteratorMode) -> DBIterator<'a>;

    /// Iterator for user commands via blockchain length
    fn user_commands_height_iterator<'a>(&'a self, mode: IteratorMode) -> DBIterator<'a>;

    /// Iterator for user commands by sender via block height
    fn txn_from_height_iterator<'a>(&'a self, mode: IteratorMode) -> DBIterator<'a>;

    /// Iterator for user commands by sender via global slot
    fn txn_from_slot_iterator<'a>(&'a self, mode: IteratorMode) -> DBIterator<'a>;

    /// Iterator for user commands by sender via block height
    fn txn_to_height_iterator<'a>(&'a self, mode: IteratorMode) -> DBIterator<'a>;

    /// Iterator for user commands by receiver via global slot
    fn txn_to_slot_iterator<'a>(&'a self, mode: IteratorMode) -> DBIterator<'a>;

    /////////////////////////
    // User command counts //
    /////////////////////////

    /// Get the number of blocks in which `pk` has transactions
    fn get_pk_num_user_commands_blocks(&self, pk: &PublicKey) -> anyhow::Result<Option<u32>>;

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

/// Global slot number from `key` in [user_commands_iterator]
/// - keep the first 4 bytes
/// - used for global slot & block height
pub fn user_commands_iterator_u32_prefix(key: &[u8]) -> u32 {
    from_be_bytes(key[..4].to_vec())
}

/// Transaction hash from `key` in [user_commands_iterator]
/// - discard the first 4 bytes
pub fn user_commands_iterator_txn_hash(key: &[u8]) -> anyhow::Result<String> {
    String::from_utf8(key[4..(4 + TXN_HASH_LEN)].to_vec())
        .map_err(|e| anyhow!("Error reading txn hash: {e}"))
}

/// State hash from `key` in [user_commands_iterator]
/// - discard the first 4 + [TXN_HASH_LEN] bytes
pub fn user_commands_iterator_state_hash(key: &[u8]) -> anyhow::Result<BlockHash> {
    BlockHash::from_bytes(&key[(4 + TXN_HASH_LEN)..])
        .map_err(|e| anyhow!("Error reading state hash: {e}"))
}
