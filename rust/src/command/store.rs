//! User command store trait

use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::{precomputed::PrecomputedBlock, store::DbBlockUpdate},
    command::{
        signed::{SignedCommandWithData, TxnHash},
        UserCommandWithStatus,
    },
};
use anyhow::Result;
use speedb::{DBIterator, IteratorMode, WriteBatch};
use std::path::PathBuf;

pub trait UserCommandStore {
    /// Index user commands (transactions) from the given block on:
    /// public keys, transaction hash, and state hashes
    fn add_user_commands_batch(
        &self,
        block: &PrecomputedBlock,
        batch: &mut WriteBatch,
    ) -> Result<()>;

    /// Set user commands for the given block
    fn set_block_user_commands_batch(
        &self,
        block: &PrecomputedBlock,
        batch: &mut WriteBatch,
    ) -> Result<()>;

    /// Get indexed user commands from the given block
    fn get_block_user_commands(
        &self,
        state_hash: &StateHash,
    ) -> Result<Option<Vec<UserCommandWithStatus>>>;

    /// Get user command by its hash & index
    fn get_user_command(
        &self,
        txn_hash: &TxnHash,
        index: u32,
    ) -> Result<Option<SignedCommandWithData>>;

    /// Get user command by its hash & containing block
    fn get_user_command_state_hash(
        &self,
        txn_hash: &TxnHash,
        state_hash: &StateHash,
    ) -> Result<Option<SignedCommandWithData>>;

    /// Get indexed user commands involving the public key as a sender or
    /// receiver
    fn get_user_commands_for_public_key(
        &self,
        pk: &PublicKey,
    ) -> Result<Option<Vec<SignedCommandWithData>>>;

    /// Get user commands for the public key with number and/or state hash
    /// bounds
    fn get_user_commands_with_bounds(
        &self,
        pk: &PublicKey,
        start_state_hash: &StateHash,
        end_state_hash: &StateHash,
    ) -> Result<Vec<SignedCommandWithData>>;

    /// Set block containing `txn_hash`
    fn set_user_command_state_hash_batch(
        &self,
        state_hash: StateHash,
        txn_hash: &TxnHash,
        batch: &mut WriteBatch,
    ) -> Result<()>;

    /// Get state hashes of blocks containing `txn_hash` in block sorted order
    fn get_user_command_state_hashes(&self, txn_hash: &TxnHash) -> Result<Option<Vec<StateHash>>>;

    /// Get number of blocks containing `txn_hash`
    fn get_user_commands_num_containing_blocks(&self, txn_hash: &TxnHash) -> Result<Option<u32>>;

    /// Write the account's user commands to a CSV file
    fn write_user_commands_csv(&self, pk: &PublicKey, path: Option<PathBuf>) -> Result<PathBuf>;

    ///////////////
    // Iterators //
    ///////////////

    /// Iterator for user commands via global slot
    fn user_commands_slot_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Iterator for user commands via blockchain length
    fn user_commands_height_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Iterator for user commands by sender via block height
    fn txn_from_height_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Iterator for user commands by sender via global slot
    fn txn_from_slot_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Iterator for user commands by sender via block height
    fn txn_to_height_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Iterator for user commands by receiver via global slot
    fn txn_to_slot_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Iterator for zkapp commands via global slot
    fn zkapp_commands_slot_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Iterator for zkapp commands via blockchain length
    fn zkapp_commands_height_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /////////////////////////
    // User command counts //
    /////////////////////////

    /// Get the number of blocks in which `pk` has transactions
    fn get_pk_num_user_commands_blocks(&self, pk: &PublicKey) -> Result<Option<u32>>;

    /// Increment user commands per epoch count
    fn increment_user_commands_epoch_count(&self, epoch: u32) -> Result<()>;

    /// Increment zkapp commands per epoch count
    fn increment_zkapp_commands_epoch_count(&self, epoch: u32) -> Result<()>;

    /// Get user commands per epoch count
    fn get_user_commands_epoch_count(&self, epoch: Option<u32>) -> Result<u32>;

    /// Get zkapp commands per epoch count
    fn get_zkapp_commands_epoch_count(&self, epoch: Option<u32>) -> Result<u32>;

    /// Increment user commands total count
    fn increment_user_commands_total_count(&self) -> Result<()>;

    /// Increment zkapp commands total count
    fn increment_zkapp_commands_total_count(&self) -> Result<()>;

    /// Get user commands total count
    fn get_user_commands_total_count(&self) -> Result<u32>;

    /// Get zkapp commands total count
    fn get_zkapp_commands_total_count(&self) -> Result<u32>;

    /// Increment user commands per epoch per account count
    fn increment_user_commands_pk_epoch_count(&self, pk: &PublicKey, epoch: u32) -> Result<()>;

    /// Increment zkapp commands per epoch per account count
    fn increment_zkapp_commands_pk_epoch_count(&self, pk: &PublicKey, epoch: u32) -> Result<()>;

    /// Get user commands per epoch per account count
    fn get_user_commands_pk_epoch_count(&self, pk: &PublicKey, epoch: Option<u32>) -> Result<u32>;

    /// Get zkapp commands per epoch per account count
    fn get_zkapp_commands_pk_epoch_count(&self, pk: &PublicKey, epoch: Option<u32>) -> Result<u32>;

    /// Increment user commands per account total
    fn increment_user_commands_pk_total_count(&self, pk: &PublicKey) -> Result<()>;

    /// Increment zkapp commands per account total
    fn increment_zkapp_commands_pk_total_count(&self, pk: &PublicKey) -> Result<()>;

    /// Get user commands per account total
    fn get_user_commands_pk_total_count(&self, pk: &PublicKey) -> Result<u32>;

    /// Get zkapp commands per account total
    fn get_zkapp_commands_pk_total_count(&self, pk: &PublicKey) -> Result<u32>;

    /// Increment user commands per block
    fn set_block_user_commands_count_batch(
        &self,
        state_hash: &StateHash,
        count: u32,
        batch: &mut WriteBatch,
    ) -> Result<()>;

    /// Increment zkapp commands per block
    fn set_block_zkapp_commands_count_batch(
        &self,
        state_hash: &StateHash,
        count: u32,
        batch: &mut WriteBatch,
    ) -> Result<()>;

    /// Get user commands per block
    fn get_block_user_commands_count(&self, state_hash: &StateHash) -> Result<Option<u32>>;

    /// Get zkapp commands per block
    fn get_block_zkapp_commands_count(&self, state_hash: &StateHash) -> Result<Option<u32>>;

    /// Increment user commands counts given `command` in `epoch`
    fn increment_user_commands_counts(
        &self,
        command: &UserCommandWithStatus,
        epoch: u32,
    ) -> Result<()>;

    /// Increment zkapp commands counts given `command` in `epoch`
    fn increment_zkapp_commands_counts(
        &self,
        command: &UserCommandWithStatus,
        epoch: u32,
    ) -> Result<()>;

    /// Get applied user commands count
    fn get_applied_user_commands_count(&self) -> Result<u32>;

    /// Get applied zkapp commands count
    fn get_applied_zkapp_commands_count(&self) -> Result<u32>;

    /// Get failed user commands count
    fn get_failed_user_commands_count(&self) -> Result<u32>;

    /// Get failed zkapp commands count
    fn get_failed_zkapp_commands_count(&self) -> Result<u32>;

    /// Increment applied user commands count
    fn increment_applied_user_commands_count(&self, num: u32) -> Result<()>;

    /// Increment applied zkapp commands count
    fn increment_applied_zkapp_commands_count(&self, num: u32) -> Result<()>;

    /// Increment applied user commands count
    fn increment_failed_user_commands_count(&self, num: u32) -> Result<()>;

    /// Increment applied zkapp commands count
    fn increment_failed_zkapp_commands_count(&self, num: u32) -> Result<()>;

    /// Decrement failed user commands count
    fn decrement_failed_user_commands_count(&self, num: u32) -> Result<()>;

    /// Decrement failed zkapp commands count
    fn decrement_failed_zkapp_commands_count(&self, num: u32) -> Result<()>;

    /// Decrement applied user commands count
    fn decrement_applied_user_commands_count(&self, num: u32) -> Result<()>;

    /// Decrement applied zkapp commands count
    fn decrement_applied_zkapp_commands_count(&self, num: u32) -> Result<()>;

    /// Get canonical user commands count
    fn get_canonical_user_commands_count(&self) -> Result<u32>;

    /// Get canonical zkapp commands count
    fn get_canonical_zkapp_commands_count(&self) -> Result<u32>;

    /// Increment canonical user commands count
    fn increment_canonical_user_commands_count(&self, num: u32) -> Result<()>;

    /// Increment canonical zkapp commands count
    fn increment_canonical_zkapp_commands_count(&self, num: u32) -> Result<()>;

    /// Decrement canonical user commands count
    fn decrement_canonical_user_commands_count(&self, num: u32) -> Result<()>;

    /// Decrement canonical zkapp commands count
    fn decrement_canonical_zkapp_commands_count(&self, num: u32) -> Result<()>;

    /// Get applied canonical user commands count
    fn get_applied_canonical_user_commands_count(&self) -> Result<u32>;

    /// Get applied canonical zkapp commands count
    fn get_applied_canonical_zkapp_commands_count(&self) -> Result<u32>;

    /// Increment canonical user commands count
    fn increment_applied_canonical_user_commands_count(&self, num: u32) -> Result<()>;

    /// Increment canonical zkapp commands count
    fn increment_applied_canonical_zkapp_commands_count(&self, num: u32) -> Result<()>;

    /// Decrement canonical user commands count
    fn decrement_applied_canonical_user_commands_count(&self, num: u32) -> Result<()>;

    /// Decrement canonical zkapp commands count
    fn decrement_applied_canonical_zkapp_commands_count(&self, num: u32) -> Result<()>;

    /// Get failed canonical user commands count
    fn get_failed_canonical_user_commands_count(&self) -> Result<u32>;

    /// Get failed canonical zkapp commands count
    fn get_failed_canonical_zkapp_commands_count(&self) -> Result<u32>;

    /// Increment canonical user commands count
    fn increment_failed_canonical_user_commands_count(&self, num: u32) -> Result<()>;

    /// Increment canonical zkapp commands count
    fn increment_failed_canonical_zkapp_commands_count(&self, num: u32) -> Result<()>;

    /// decrement canonical user commands count
    fn decrement_failed_canonical_user_commands_count(&self, num: u32) -> Result<()>;

    /// decrement canonical zkapp commands count
    fn decrement_failed_canonical_zkapp_commands_count(&self, num: u32) -> Result<()>;

    /// Update user command counts from `DbBlockUpdate`
    fn update_user_commands(&self, block: &DbBlockUpdate) -> Result<()>;
}
