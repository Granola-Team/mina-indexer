//! Username store impl

use super::{
    column_families::ColumnFamilyHelpers,
    username::{UsernameAccountUpdate, UsernameStore, UsernameUpdate},
    DbUpdate, IndexerStore,
};
use crate::{
    base::{public_key::PublicKey, state_hash::StateHash, username::Username},
    block::store::{BlockUpdate, DbBlockUpdate},
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
        let count = index + 1;

        self.database
            .put_cf(self.username_num_cf(), pk.0.as_bytes(), count.to_be_bytes())?;

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
        trace!("Getting username '{}' accounts", username);

        Ok(self
            .database
            .get_cf(self.username_pk_cf(), username.as_bytes())?
            .and_then(|bytes| serde_json::from_slice(&bytes).expect("username pks")))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        base::{public_key::PublicKey, username::Username},
        store::{username::UsernameStore, IndexerStore},
    };
    use quickcheck::{Arbitrary, Gen};
    use std::collections::BTreeSet;
    use tempfile::TempDir;

    const GEN_SIZE: usize = 1000;

    fn create_indexer_store() -> anyhow::Result<IndexerStore> {
        let temp_dir = TempDir::with_prefix(std::env::current_dir()?)?;
        IndexerStore::new(temp_dir.path(), true)
    }

    #[test]
    fn username_store() -> anyhow::Result<()> {
        let store = create_indexer_store()?;

        let g = &mut Gen::new(GEN_SIZE);
        let pk0 = PublicKey::arbitrary(g);
        let pk1 = PublicKey::arbitrary_not(g, &Some(pk0.clone()));

        // no usernames initially
        assert!(store.get_pk_num_username_updates(&pk0)?.is_none());
        assert!(store.get_pk_num_username_updates(&pk1)?.is_none());

        assert!(store.get_username(&pk0)?.is_none());
        assert!(store.get_username(&pk1)?.is_none());

        // add pk0 username
        let username0 = Username::arbitrary(g);
        store.add_username(pk0.clone(), &username0)?;

        assert_eq!(store.get_pk_num_username_updates(&pk0)?.unwrap(), 1);
        assert_eq!(store.get_pk_username(&pk0, 0)?.unwrap(), username0);
        assert_eq!(store.get_username(&pk0)?.unwrap(), username0);
        assert_eq!(
            store.get_username_pks(&username0.0)?.unwrap(),
            BTreeSet::from([pk0.clone()])
        );

        // add pk1 username (same as pk0)
        store.add_username(pk1.clone(), &username0)?;

        assert_eq!(store.get_pk_num_username_updates(&pk1)?.unwrap(), 1);
        assert_eq!(store.get_pk_username(&pk1, 0)?.unwrap(), username0);
        assert_eq!(store.get_username(&pk1)?.unwrap(), username0);
        assert_eq!(
            store.get_username_pks(&username0.0)?.unwrap(),
            BTreeSet::from([pk0.clone(), pk1.clone()])
        );

        // remove pk1 username
        store.remove_username(&pk1)?;

        assert!(store.get_username(&pk1)?.is_none());
        assert_eq!(store.get_pk_num_username_updates(&pk1)?.unwrap(), 0);
        assert_eq!(
            store.get_username_pks(&username0.0)?.unwrap(),
            BTreeSet::from([pk0.clone()])
        );

        // add pk1 username (different from pk0)
        let username1 = Username::arbitrary_not(g, &username0);
        store.add_username(pk1.clone(), &username1)?;

        assert_eq!(store.get_pk_num_username_updates(&pk1)?.unwrap(), 1);
        assert_eq!(store.get_pk_username(&pk1, 0)?.unwrap(), username1);
        assert_eq!(store.get_username(&pk1)?.unwrap(), username1);
        assert_eq!(
            store.get_username_pks(&username1.0)?.unwrap(),
            BTreeSet::from([pk1.clone()])
        );

        Ok(())
    }

    #[test]
    fn off_chain_usernames() -> anyhow::Result<()> {
        let store = create_indexer_store()?;

        let pk = PublicKey::new("B62qpge4uMq4Vv5Rvc8Gw9qSquUYd6xoW1pz7HQkMSHm6h1o7pvLPAN")?;
        let username = Username::new("MinaExplorer")?;

        assert_eq!(store.get_username(&pk)?.unwrap(), username);
        assert_eq!(store.get_pk_num_username_updates(&pk)?.unwrap(), 1);
        assert_eq!(
            store.get_username_pks(&username.0)?.unwrap(),
            BTreeSet::from([pk])
        );

        Ok(())
    }
}
