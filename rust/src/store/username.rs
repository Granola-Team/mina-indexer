use crate::{
    block::BlockHash,
    ledger::{public_key::PublicKey, username::Username},
};
use std::collections::HashMap;

pub trait UsernameStore {
    /// Get the username associated with `pk`
    fn get_username(&self, pk: &PublicKey) -> anyhow::Result<Option<Username>>;

    /// Set `pk`'s username to `username`
    fn set_username(&self, pk: &PublicKey, username: Username) -> anyhow::Result<()>;

    /// Set the user name updates in the block
    fn set_block_username_updates(
        &self,
        state_hash: &BlockHash,
        username_updates: &HashMap<PublicKey, Username>,
    ) -> anyhow::Result<()>;

    fn get_block_username_updates(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<HashMap<PublicKey, Username>>>;
}
