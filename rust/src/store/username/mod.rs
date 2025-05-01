//! Username store trait

use super::DbUpdate;
use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::store::DbBlockUpdate,
    ledger::username::Username,
    store::Result,
};
use serde::{Deserialize, Serialize};
use speedb::WriteBatch;
use std::collections::{BTreeSet, HashMap};

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct UsernameUpdate(pub HashMap<PublicKey, Username>);

pub type UsernameAccountUpdate = DbUpdate<UsernameUpdate>;

pub trait UsernameStore {
    /// Get the username associated with pk
    fn get_username(&self, pk: &PublicKey) -> Result<Option<Username>>;

    /// Add pk's username
    fn add_username(&self, pk: PublicKey, username: &Username) -> Result<()>;

    /// Remove pk's username
    fn remove_username(&self, pk: &PublicKey) -> Result<()>;

    /// Get number of pk username updates
    fn get_pk_num_username_updates(&self, pk: &PublicKey) -> Result<Option<u32>>;

    /// Get the specified username
    fn get_pk_username(&self, pk: &PublicKey, index: u32) -> Result<Option<Username>>;

    /// Set the user name updates in the block
    fn set_block_username_updates_batch(
        &self,
        state_hash: &StateHash,
        username_updates: &UsernameUpdate,
        batch: &mut WriteBatch,
    ) -> Result<()>;

    /// Get the block's username updates
    fn get_block_username_updates(&self, state_hash: &StateHash) -> Result<Option<UsernameUpdate>>;

    /// Get the accounts associated with the given username
    fn get_username_pks(&self, username: &str) -> Result<Option<BTreeSet<PublicKey>>>;

    /// Update block usernames
    fn update_block_usernames(&self, blocks: &DbBlockUpdate) -> Result<()>;

    /// Update usernames
    fn update_usernames(&self, update: UsernameAccountUpdate) -> Result<()>;
}
