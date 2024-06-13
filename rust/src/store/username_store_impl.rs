use super::{username::UsernameStore, IndexerStore};
use crate::{
    block::BlockHash,
    ledger::{public_key::PublicKey, username::Username},
    store::column_families::ColumnFamilyHelpers,
};
use log::trace;
use std::collections::HashMap;

impl UsernameStore for IndexerStore {
    fn get_username(&self, pk: &PublicKey) -> anyhow::Result<Option<Username>> {
        trace!("Getting {pk} username");
        Ok(self
            .database
            .get_cf(self.usernames_cf(), pk.0.as_bytes())?
            .and_then(|bytes| Username::from_bytes(bytes).ok()))
    }

    fn set_username(&self, pk: &PublicKey, username: Username) -> anyhow::Result<()> {
        trace!("Setting {pk} username {username}");
        Ok(self
            .database
            .put_cf(self.usernames_cf(), pk.0.as_bytes(), username.0.as_bytes())?)
    }

    fn set_block_username_updates(
        &self,
        state_hash: &BlockHash,
        username_updates: &HashMap<PublicKey, Username>,
    ) -> anyhow::Result<()> {
        trace!("Setting block username updates {state_hash}");
        Ok(self.database.put_cf(
            self.usernames_per_block_cf(),
            state_hash.0.as_bytes(),
            serde_json::to_vec(username_updates)?,
        )?)
    }

    fn get_block_username_updates(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<HashMap<PublicKey, Username>>> {
        trace!("Getting block username updates {state_hash}");
        Ok(self
            .database
            .get_pinned_cf(self.usernames_per_block_cf(), state_hash.0.as_bytes())?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok()))
    }
}
