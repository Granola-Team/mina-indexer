use super::DbUpdate;
use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::store::DbBlockUpdate,
    ledger::username::Username,
};
use serde::{Deserialize, Serialize};
use speedb::WriteBatch;
use std::collections::HashMap;

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct UsernameUpdate(pub HashMap<PublicKey, Username>);

pub type UsernameAccountUpdate = DbUpdate<UsernameUpdate>;

pub trait UsernameStore {
    /// Get the username associated with `pk`
    fn get_username(&self, pk: &PublicKey) -> anyhow::Result<Option<Username>>;

    /// Get number of pk username updates
    fn get_pk_num_username_updates(&self, pk: &PublicKey) -> anyhow::Result<Option<u32>>;

    /// Get pk's index-th username
    fn get_pk_username(&self, pk: &PublicKey, index: u32) -> anyhow::Result<Option<Username>>;

    /// Set the user name updates in the block
    fn set_block_username_updates_batch(
        &self,
        state_hash: &StateHash,
        username_updates: &UsernameUpdate,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()>;

    /// Get the block's username updates
    fn get_block_username_updates(
        &self,
        state_hash: &StateHash,
    ) -> anyhow::Result<Option<UsernameUpdate>>;

    /// Update block usernames
    fn update_block_usernames(&self, blocks: &DbBlockUpdate) -> anyhow::Result<()>;

    /// Update usernames
    fn update_usernames(&self, update: UsernameAccountUpdate) -> anyhow::Result<()>;
}
