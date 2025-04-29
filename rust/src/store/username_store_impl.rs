use super::{
    column_families::ColumnFamilyHelpers,
    username::{UsernameAccountUpdate, UsernameStore, UsernameUpdate},
    DbUpdate, IndexerStore,
};
use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::store::{BlockUpdate, DbBlockUpdate},
    ledger::username::Username,
    store::Result,
    utility::store::{common::from_be_bytes, username::username_key},
};
use log::{error, trace};
use speedb::WriteBatch;
use std::collections::BTreeSet;

impl UsernameStore for IndexerStore {
    fn get_username(&self, pk: &PublicKey) -> Result<Option<Username>> {
        trace!("Getting {pk} username");

        if let Ok(Some(num)) = self.get_pk_num_username_updates(pk) {
            if num == 0 {
                return Ok(None);
            }

            return self.get_pk_username(pk, num - 1);
        }

        Ok(None)
    }

    fn add_username(&self, pk: PublicKey, username: &Username) -> Result<()> {
        trace!("Adding username: {} -> {}", pk, username);

        let index = self.get_pk_num_username_updates(&pk)?.unwrap_or_default();
        self.database.put_cf(
            self.username_num_cf(),
            pk.0.as_bytes(),
            (index + 1).to_be_bytes(),
        )?;

        self.database.put_cf(
            self.username_cf(),
            username_key(&pk, index),
            username.0.as_bytes(),
        )?;

        // update username pks
        let mut pks = self.get_username_pks(&username.0)?.unwrap_or_default();
        pks.insert(pk);

        self.database.put_cf(
            self.username_pk_cf(),
            username.0.as_bytes(),
            serde_json::to_vec(&pks)?,
        )?;

        Ok(())
    }

    fn remove_username(&self, pk: &PublicKey) -> Result<()> {
        trace!("Removing username {}", pk);

        if let Some(num) = self.get_pk_num_username_updates(pk)? {
            assert!(num > 0, "No username to unapply {}", pk);

            // update account usernames
            let index = num - 1;
            let username = self.get_username(pk)?.expect("remove username");

            self.database
                .delete_cf(self.username_cf(), username_key(pk, index))?;
            self.database
                .put_cf(self.username_num_cf(), pk.0.as_bytes(), index.to_be_bytes())?;

            // update username accounts
            let mut pks = self.get_username_pks(&username.0)?.unwrap_or_default();
            pks.remove(pk);

            self.database.put_cf(
                self.username_pk_cf(),
                username.0.as_bytes(),
                serde_json::to_vec(&pks)?,
            )?;
        } else {
            error!("Invalid username num {}", pk);
        }

        Ok(())
    }

    fn set_block_username_updates_batch(
        &self,
        state_hash: &StateHash,
        username_updates: &UsernameUpdate,
        batch: &mut WriteBatch,
    ) -> Result<()> {
        trace!("Setting block username updates {state_hash}");

        batch.put_cf(
            self.usernames_per_block_cf(),
            state_hash.0.as_bytes(),
            serde_json::to_vec(username_updates)?,
        );

        Ok(())
    }

    fn get_block_username_updates(&self, state_hash: &StateHash) -> Result<Option<UsernameUpdate>> {
        trace!("Getting block username updates {state_hash}");
        Ok(self
            .database
            .get_cf(self.usernames_per_block_cf(), state_hash.0.as_bytes())?
            .map(|bytes| serde_json::from_slice(&bytes).expect("block username updates")))
    }

    fn update_block_usernames(&self, blocks: &DbBlockUpdate) -> Result<()> {
        trace!("Updating block usernames {:?}", blocks);

        let username_updates = DbUpdate {
            apply: blocks
                .apply
                .iter()
                .map(|BlockUpdate { state_hash: a, .. }| {
                    self.get_block_username_updates(a)
                        .unwrap()
                        .expect("username update apply")
                })
                .collect(),
            unapply: blocks
                .unapply
                .iter()
                .map(|BlockUpdate { state_hash: u, .. }| {
                    self.get_block_username_updates(u)
                        .unwrap()
                        .expect("username update unapply")
                })
                .collect(),
        };

        self.update_usernames(username_updates)
    }

    fn update_usernames(&self, update: UsernameAccountUpdate) -> Result<()> {
        trace!("Updating usernames {:?}", update);

        // unapply
        for updates in update.unapply {
            for pk in updates.0.keys() {
                self.remove_username(pk)?;
            }
        }

        // apply
        for updates in update.apply {
            for (pk, username) in updates.0 {
                self.add_username(pk, &username)?;
            }
        }

        Ok(())
    }

    fn get_pk_username(&self, pk: &PublicKey, index: u32) -> Result<Option<Username>> {
        trace!("Getting username {} index {}", pk, index);

        Ok(self
            .database
            .get_cf(self.username_cf(), username_key(pk, index))?
            .map(|bytes| Username::from_bytes(bytes).expect("username")))
    }

    fn get_pk_num_username_updates(&self, pk: &PublicKey) -> Result<Option<u32>> {
        trace!("Getting username update count {}", pk);

        Ok(self
            .database
            .get_cf(self.username_num_cf(), pk.0.as_bytes())?
            .map(from_be_bytes))
    }

    fn get_username_pks(&self, username: &str) -> Result<Option<BTreeSet<PublicKey>>> {
        trace!("Getting username {} accounts", username);

        Ok(self
            .database
            .get_cf(self.username_pk_cf(), username.as_bytes())?
            .and_then(|bytes| serde_json::from_slice(&bytes).expect("username pks")))
    }
}
