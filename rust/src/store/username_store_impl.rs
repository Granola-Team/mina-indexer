use super::{
    column_families::ColumnFamilyHelpers,
    username::{UsernameAccountUpdate, UsernameStore, UsernameUpdate},
    DbUpdate, IndexerStore,
};
use crate::{
    block::{store::DbBlockUpdate, BlockHash},
    ledger::{public_key::PublicKey, username::Username},
    utility::db::{from_be_bytes, pk_index_key, to_be_bytes},
};
use log::{error, trace};
use speedb::WriteBatch;
use std::collections::HashMap;

impl UsernameStore for IndexerStore {
    fn get_username(&self, pk: &PublicKey) -> anyhow::Result<Option<Username>> {
        trace!("Getting {pk} username");
        if let Ok(Some(index)) = self.get_pk_num_username_updates(pk) {
            return self.get_pk_username(pk, index);
        }
        Ok(None)
    }

    fn set_block_username_updates_batch(
        &self,
        state_hash: &BlockHash,
        username_updates: &UsernameUpdate,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()> {
        trace!("Setting block username updates {state_hash}");
        batch.put_cf(
            self.usernames_per_block_cf(),
            state_hash.0.as_bytes(),
            serde_json::to_vec(username_updates)?,
        );
        Ok(())
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

    fn update_block_usernames(&self, blocks: &DbBlockUpdate) -> anyhow::Result<()> {
        let username_updates = DbUpdate {
            apply: blocks
                .apply
                .iter()
                .map(|(a, _)| {
                    UsernameUpdate(self.get_block_username_updates(a).ok().flatten().unwrap())
                })
                .collect(),
            unapply: blocks
                .unapply
                .iter()
                .map(|(u, _)| {
                    UsernameUpdate(self.get_block_username_updates(u).ok().flatten().unwrap())
                })
                .collect(),
        };
        self.update_usernames(username_updates)
    }

    fn update_usernames(&self, update: UsernameAccountUpdate) -> anyhow::Result<()> {
        trace!("Updating usernames");

        // unapply
        for updates in update.unapply {
            for pk in updates.0.keys() {
                if let Some(num) = self.get_pk_num_username_updates(pk)? {
                    // decr pk num username updates
                    if num == 0 {
                        // remove pk
                        self.database
                            .delete_cf(self.username_pk_num_cf(), pk.0.as_bytes())?;
                    } else {
                        // decrement username update num
                        self.database.put_cf(
                            self.username_pk_num_cf(),
                            pk.0.as_bytes(),
                            to_be_bytes(num - 1),
                        )?;
                    }

                    // drop last username update
                    self.database
                        .delete_cf(self.username_pk_index_cf(), pk_index_key(pk.clone(), num))?;
                } else {
                    error!("Invalid username pk num {pk}");
                }
            }
        }

        // apply
        for updates in update.apply {
            for (pk, username) in updates.0 {
                let index = if let Some(num) = self.get_pk_num_username_updates(&pk)? {
                    // incr pk num username updates
                    num + 1
                } else {
                    0
                };

                // update num
                self.database.put_cf(
                    self.username_pk_num_cf(),
                    pk.0.as_bytes(),
                    index.to_be_bytes(),
                )?;

                // set current username
                self.database.put_cf(
                    self.username_pk_index_cf(),
                    pk_index_key(pk, index),
                    username.0.as_bytes(),
                )?;
            }
        }
        Ok(())
    }

    fn get_pk_username(&self, pk: &PublicKey, index: u32) -> anyhow::Result<Option<Username>> {
        trace!("Getting pk's {index}th username {pk}");
        Ok(self
            .database
            .get_cf(self.username_pk_index_cf(), pk_index_key(pk.clone(), index))?
            .and_then(|bytes| Username::from_bytes(bytes).ok()))
    }

    fn get_pk_num_username_updates(&self, pk: &PublicKey) -> anyhow::Result<Option<u32>> {
        trace!("Getting pk's number of username updates {pk}");
        Ok(self
            .database
            .get_cf(self.username_pk_num_cf(), pk.0.as_bytes())?
            .map(from_be_bytes))
    }
}
