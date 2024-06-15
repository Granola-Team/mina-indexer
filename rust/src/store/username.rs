use super::DBUpdate;
use crate::{
    block::BlockHash,
    ledger::{public_key::PublicKey, username::Username},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Default, Serialize, Deserialize)]
pub struct UsernameUpdate(pub HashMap<PublicKey, Username>);

pub type UsernameAccountUpdate = DBUpdate<UsernameUpdate>;

pub trait UsernameStore {
    /// Get the username associated with `pk`
    fn get_username(&self, pk: &PublicKey) -> anyhow::Result<Option<Username>>;

    /// Get number of pk username updates
    fn get_pk_num_username_updates(&self, pk: &PublicKey) -> anyhow::Result<Option<u32>>;

    /// Get pk's index-th username
    fn get_pk_username(&self, pk: &PublicKey, index: u32) -> anyhow::Result<Option<Username>>;

    /// Set the user name updates in the block
    fn set_block_username_updates(
        &self,
        state_hash: &BlockHash,
        username_updates: &UsernameUpdate,
    ) -> anyhow::Result<()>;

    fn get_block_username_updates(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<HashMap<PublicKey, Username>>>;

    /// Generate account username updates when the best tip changes
    fn reorg_username_updates(
        &self,
        old_best_tip: &BlockHash,
        new_best_tip: &BlockHash,
    ) -> anyhow::Result<UsernameAccountUpdate>;

    /// Update usernames
    fn update_usernames(&self, update: UsernameAccountUpdate) -> anyhow::Result<()>;
}
