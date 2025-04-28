//! Zkapp store trait implementation

use super::{zkapp::ZkappStore, IndexerStore, Result};
use crate::{
    base::public_key::PublicKey,
    ledger::{
        account::{Permissions, Timing},
        token::{TokenAddress, TokenSymbol},
    },
    mina_blocks::v2::{VerificationKey, ZkappState, ZkappUri},
    store::column_families::ColumnFamilyHelpers,
    utility::store::{
        common::from_be_bytes,
        zkapp::{
            zkapp_permissions_key, zkapp_permissions_num_key, zkapp_state_key, zkapp_state_num_key,
            zkapp_timing_key, zkapp_timing_num_key, zkapp_token_symbol_key,
            zkapp_token_symbol_num_key, zkapp_uri_key, zkapp_uri_num_key,
            zkapp_verification_key_key, zkapp_verification_key_num_key,
        },
    },
};
use log::trace;

pub mod action_store_impl;
pub mod event_store_impl;
pub mod token_store_impl;

impl ZkappStore for IndexerStore {
    ///////////////
    // app state //
    ///////////////

    fn get_zkapp_state_num(&self, token: &TokenAddress, pk: &PublicKey) -> Result<Option<u32>> {
        trace!("Getting zkapp state count for token {} pk {}", token, pk);

        Ok(self
            .database
            .get_cf(self.zkapp_state_num_cf(), zkapp_state_num_key(token, pk))?
            .map(from_be_bytes))
    }

    fn get_zkapp_state(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        index: u32,
    ) -> Result<Option<ZkappState>> {
        trace!(
            "Getting zkapp state for token {} pk {} index {}",
            token,
            pk,
            index
        );

        Ok(self
            .database
            .get_cf(self.zkapp_state_cf(), zkapp_state_key(token, pk, index))?
            .map(|bytes| serde_json::from_slice(&bytes).expect("zkapp state")))
    }

    fn add_zkapp_state(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        app_state: &ZkappState,
    ) -> Result<()> {
        trace!(
            "Adding zkapp state for token {} pk {}: {:?}",
            token,
            pk,
            app_state
        );

        // get index & update count
        let index = self.get_zkapp_state_num(token, pk)?.unwrap_or_default();
        self.database.put_cf(
            self.zkapp_state_num_cf(),
            zkapp_state_num_key(token, pk),
            (index + 1).to_be_bytes(),
        )?;

        // write entry
        Ok(self.database.put_cf(
            self.zkapp_state_cf(),
            zkapp_state_key(token, pk, index),
            serde_json::to_vec(app_state)?,
        )?)
    }

    fn remove_last_zkapp_state(&self, token: &TokenAddress, pk: &PublicKey) -> Result<ZkappState> {
        trace!("Removing last zkapp state for token {} pk {}", token, pk);

        let count = self
            .get_zkapp_state_num(token, pk)?
            .expect("zkapp state count");
        assert_ne!(count, 0);

        let index = count - 1;
        let zkapp_state = self
            .get_zkapp_state(token, pk, index)?
            .expect("last zkapp state");

        // delete entry
        self.database
            .delete_cf(self.zkapp_state_cf(), zkapp_state_key(token, pk, index))?;

        // update count
        self.database.put_cf(
            self.zkapp_state_num_cf(),
            zkapp_state_num_key(token, pk),
            index.to_be_bytes(),
        )?;

        Ok(zkapp_state)
    }

    /////////////////
    // permissions //
    /////////////////

    fn get_zkapp_permissions_num(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
    ) -> Result<Option<u32>> {
        trace!(
            "Getting zkapp permissions count for token {} pk {}",
            token,
            pk
        );

        Ok(self
            .database
            .get_cf(
                self.zkapp_permissions_num_cf(),
                zkapp_permissions_num_key(token, pk),
            )?
            .map(from_be_bytes))
    }

    fn get_zkapp_permissions(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        index: u32,
    ) -> Result<Option<Permissions>> {
        trace!(
            "Getting zkapp permissions for token {} pk {} index {}",
            token,
            pk,
            index
        );

        Ok(self
            .database
            .get_cf(
                self.zkapp_permissions_cf(),
                zkapp_permissions_key(token, pk, index),
            )?
            .map(|bytes| serde_json::from_slice(&bytes).expect("zkapp permissions")))
    }

    fn add_zkapp_permissions(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        permissions: &Permissions,
    ) -> Result<()> {
        trace!(
            "Adding zkapp permissions for token {} pk {}: {:?}",
            token,
            pk,
            permissions
        );

        // get index & update count
        let index = self
            .get_zkapp_permissions_num(token, pk)?
            .unwrap_or_default();
        self.database.put_cf(
            self.zkapp_permissions_num_cf(),
            zkapp_permissions_num_key(token, pk),
            (index + 1).to_be_bytes(),
        )?;

        // write entry
        Ok(self.database.put_cf(
            self.zkapp_permissions_cf(),
            zkapp_permissions_key(token, pk, index),
            serde_json::to_vec(permissions)?,
        )?)
    }

    fn remove_last_zkapp_permissions(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
    ) -> Result<Permissions> {
        trace!(
            "Removing last zkapp permissions for token {} pk {}",
            token,
            pk
        );

        let count = self
            .get_zkapp_permissions_num(token, pk)?
            .expect("zkapp permissions count");
        assert_ne!(count, 0);

        let index = count - 1;
        let permissions = self
            .get_zkapp_permissions(token, pk, index)?
            .expect("last zkapp permissions");

        // delete entry
        self.database.delete_cf(
            self.zkapp_permissions_cf(),
            zkapp_permissions_key(token, pk, index),
        )?;

        // update count
        self.database.put_cf(
            self.zkapp_permissions_num_cf(),
            zkapp_permissions_num_key(token, pk),
            index.to_be_bytes(),
        )?;

        Ok(permissions)
    }

    //////////////////////
    // verification key //
    //////////////////////

    fn get_zkapp_verification_key_num(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
    ) -> Result<Option<u32>> {
        trace!(
            "Getting zkapp verification key count for token {} pk {}",
            token,
            pk
        );

        Ok(self
            .database
            .get_cf(
                self.zkapp_verification_key_num_cf(),
                zkapp_verification_key_num_key(token, pk),
            )?
            .map(from_be_bytes))
    }

    fn get_zkapp_verification_key(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        index: u32,
    ) -> Result<Option<VerificationKey>> {
        trace!(
            "Getting zkapp verification key for token {} pk {} index {}",
            token,
            pk,
            index
        );

        Ok(self
            .database
            .get_cf(
                self.zkapp_verification_key_cf(),
                zkapp_verification_key_key(token, pk, index),
            )?
            .map(|bytes| serde_json::from_slice(&bytes).expect("zkapp permissions")))
    }

    fn add_zkapp_verification_key(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        verification_key: &VerificationKey,
    ) -> Result<()> {
        trace!(
            "Adding zkapp verification key for token {} pk {}: {:?}",
            token,
            pk,
            verification_key
        );

        // get index & update count
        let index = self
            .get_zkapp_verification_key_num(token, pk)?
            .unwrap_or_default();
        self.database.put_cf(
            self.zkapp_verification_key_num_cf(),
            zkapp_verification_key_num_key(token, pk),
            (index + 1).to_be_bytes(),
        )?;

        // write entry
        Ok(self.database.put_cf(
            self.zkapp_verification_key_cf(),
            zkapp_verification_key_key(token, pk, index),
            serde_json::to_vec(verification_key)?,
        )?)
    }

    fn remove_last_zkapp_verification_key(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
    ) -> Result<VerificationKey> {
        trace!(
            "Removing last zkapp verification key for token {} pk {}",
            token,
            pk
        );

        let count = self
            .get_zkapp_verification_key_num(token, pk)?
            .unwrap_or_default();
        assert_ne!(count, 0);

        let index = count - 1;
        let verification_key = self
            .get_zkapp_verification_key(token, pk, index)?
            .expect("last zkapp verification key");

        // delete entry
        self.database.delete_cf(
            self.zkapp_verification_key_cf(),
            zkapp_verification_key_key(token, pk, index),
        )?;

        // update count
        self.database.put_cf(
            self.zkapp_verification_key_num_cf(),
            zkapp_verification_key_num_key(token, pk),
            index.to_be_bytes(),
        )?;

        Ok(verification_key)
    }

    ///////////////
    // zkapp uri //
    ///////////////

    fn get_zkapp_uri_num(&self, token: &TokenAddress, pk: &PublicKey) -> Result<Option<u32>> {
        trace!("Getting zkapp uri count for token {} pk {}", token, pk);

        Ok(self
            .database
            .get_cf(self.zkapp_uri_num_cf(), zkapp_uri_num_key(token, pk))?
            .map(from_be_bytes))
    }

    fn get_zkapp_uri(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        index: u32,
    ) -> Result<Option<ZkappUri>> {
        trace!(
            "Getting zkapp uri for token {} pk {} index {}",
            token,
            pk,
            index
        );

        Ok(self
            .database
            .get_cf(self.zkapp_uri_cf(), zkapp_uri_key(token, pk, index))?
            .map(|bytes| String::from_utf8(bytes).expect("zkapp uri").into()))
    }

    fn add_zkapp_uri(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        zkapp_uri: &ZkappUri,
    ) -> Result<()> {
        trace!(
            "Adding zkapp uri for token {} pk {}: {:?}",
            token,
            pk,
            zkapp_uri
        );

        // get index & update count
        let index = self.get_zkapp_uri_num(token, pk)?.unwrap_or_default();
        self.database.put_cf(
            self.zkapp_uri_num_cf(),
            zkapp_uri_num_key(token, pk),
            (index + 1).to_be_bytes(),
        )?;

        // write entry
        Ok(self.database.put_cf(
            self.zkapp_uri_cf(),
            zkapp_uri_key(token, pk, index),
            zkapp_uri.0.as_bytes(),
        )?)
    }

    fn remove_last_zkapp_uri(&self, token: &TokenAddress, pk: &PublicKey) -> Result<ZkappUri> {
        trace!("Removing last zkapp uri for token {} pk {}", token, pk);

        let count = self.get_zkapp_uri_num(token, pk)?.expect("zkapp uri count");
        assert_ne!(count, 0);

        let index = count - 1;
        let zkapp_uri = self
            .get_zkapp_uri(token, pk, index)?
            .expect("last zkapp uri");

        // delete entry
        self.database
            .delete_cf(self.zkapp_uri_cf(), zkapp_uri_key(token, pk, index))?;

        // update count
        self.database.put_cf(
            self.zkapp_uri_num_cf(),
            zkapp_uri_num_key(token, pk),
            index.to_be_bytes(),
        )?;

        Ok(zkapp_uri)
    }

    //////////////////
    // token symbol //
    //////////////////

    fn get_zkapp_token_symbol_num(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
    ) -> Result<Option<u32>> {
        trace!(
            "Getting zkapp token symbol count for token {} pk {}",
            token,
            pk
        );

        Ok(self
            .database
            .get_cf(
                self.zkapp_token_symbol_num_cf(),
                zkapp_token_symbol_num_key(token, pk),
            )?
            .map(from_be_bytes))
    }

    fn get_zkapp_token_symbol(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        index: u32,
    ) -> Result<Option<TokenSymbol>> {
        trace!(
            "Getting zkapp token symbol for token {} pk {} index {}",
            token,
            pk,
            index
        );

        Ok(self
            .database
            .get_cf(
                self.zkapp_token_symbol_cf(),
                zkapp_token_symbol_key(token, pk, index),
            )?
            .map(|bytes| String::from_utf8(bytes).expect("zkapp token symbol").into()))
    }

    fn add_zkapp_token_symbol(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        token_symbol: &TokenSymbol,
    ) -> Result<()> {
        trace!(
            "Adding zkapp token symbol for token {} pk {}: {:?}",
            token,
            pk,
            token_symbol
        );

        // get index & update count
        let index = self
            .get_zkapp_token_symbol_num(token, pk)?
            .unwrap_or_default();
        self.database.put_cf(
            self.zkapp_token_symbol_num_cf(),
            zkapp_token_symbol_num_key(token, pk),
            (index + 1).to_be_bytes(),
        )?;

        // write entry
        Ok(self.database.put_cf(
            self.zkapp_token_symbol_cf(),
            zkapp_token_symbol_key(token, pk, index),
            token_symbol.0.as_bytes(),
        )?)
    }

    fn remove_last_zkapp_token_symbol(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
    ) -> Result<TokenSymbol> {
        trace!(
            "Removing last zkapp token symbol for token {} pk {}",
            token,
            pk
        );

        let count = self
            .get_zkapp_token_symbol_num(token, pk)?
            .expect("zkapp token symbol count");
        assert_ne!(count, 0);

        let index = count - 1;
        let token_symbol = self
            .get_zkapp_token_symbol(token, pk, index)?
            .expect("last zkapp token symbol");

        // delete entry
        self.database.delete_cf(
            self.zkapp_token_symbol_cf(),
            zkapp_token_symbol_key(token, pk, index),
        )?;

        // update count
        self.database.put_cf(
            self.zkapp_token_symbol_num_cf(),
            zkapp_token_symbol_num_key(token, pk),
            index.to_be_bytes(),
        )?;

        Ok(token_symbol)
    }

    ////////////
    // timing //
    ////////////

    fn get_zkapp_timing_num(&self, token: &TokenAddress, pk: &PublicKey) -> Result<Option<u32>> {
        trace!("Getting zkapp timing count for token {} pk {}", token, pk);

        Ok(self
            .database
            .get_cf(self.zkapp_timing_num_cf(), zkapp_timing_num_key(token, pk))?
            .map(from_be_bytes))
    }

    fn get_zkapp_timing(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        index: u32,
    ) -> Result<Option<Timing>> {
        trace!(
            "Getting zkapp timing for token {} pk {} index {}",
            token,
            pk,
            index
        );

        Ok(self
            .database
            .get_cf(self.zkapp_timing_cf(), zkapp_timing_key(token, pk, index))?
            .map(|bytes| serde_json::from_slice(&bytes).expect("zkapp timing")))
    }

    fn add_zkapp_timing(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        timing: &Timing,
    ) -> Result<()> {
        trace!(
            "Adding zkapp timing for token {} pk {}: {:?}",
            token,
            pk,
            timing
        );

        // get index & update count
        let index = self.get_zkapp_timing_num(token, pk)?.unwrap_or_default();
        self.database.put_cf(
            self.zkapp_timing_num_cf(),
            zkapp_timing_num_key(token, pk),
            (index + 1).to_be_bytes(),
        )?;

        // write entry
        Ok(self.database.put_cf(
            self.zkapp_timing_cf(),
            zkapp_timing_key(token, pk, index),
            serde_json::to_vec(timing)?,
        )?)
    }

    fn remove_last_zkapp_timing(&self, token: &TokenAddress, pk: &PublicKey) -> Result<Timing> {
        trace!("Removing last zkapp timing for token {} pk {}", token, pk);

        let count = self
            .get_zkapp_timing_num(token, pk)?
            .expect("zkapp timing count");
        assert_ne!(count, 0);

        let index = count - 1;
        let timing = self
            .get_zkapp_timing(token, pk, index)?
            .expect("last zkapp timing");

        // delete entry
        self.database
            .delete_cf(self.zkapp_timing_cf(), zkapp_timing_key(token, pk, index))?;

        // update count
        self.database.put_cf(
            self.zkapp_timing_num_cf(),
            zkapp_timing_num_key(token, pk),
            index.to_be_bytes(),
        )?;

        Ok(timing)
    }
}

#[cfg(all(test, feature = "tier2"))]
mod tests {
    use super::ZkappStore;
    use crate::{
        base::public_key::PublicKey,
        ledger::{
            account::{Permissions, Timing},
            token::{TokenAddress, TokenSymbol},
        },
        mina_blocks::v2::{
            zkapp::{app_state::ZkappState, verification_key::VerificationKey},
            ZkappUri,
        },
        store::IndexerStore,
    };
    use quickcheck::{Arbitrary, Gen};
    use tempfile::TempDir;

    const GEN_SIZE: usize = 1000;

    fn create_indexer_store() -> anyhow::Result<IndexerStore> {
        let temp_dir = TempDir::with_prefix(std::env::current_dir()?)?;
        let store = IndexerStore::new(temp_dir.path())?;
        Ok(store)
    }

    #[test]
    fn zkapp_state() -> anyhow::Result<()> {
        let g = &mut Gen::new(GEN_SIZE);
        let store = create_indexer_store()?;

        let pk = PublicKey::arbitrary(g);
        let token = TokenAddress::arbitrary(g);

        assert!(store.get_zkapp_state_num(&token, &pk)?.is_none());

        // add zkapp state
        let zkapp_state0 = ZkappState::arbitrary(g);
        store.add_zkapp_state(&token, &pk, &zkapp_state0)?;

        assert_eq!(store.get_zkapp_state_num(&token, &pk)?.unwrap(), 1);

        // add another zkapp state
        let zkapp_state1 = ZkappState::arbitrary(g);
        store.add_zkapp_state(&token, &pk, &zkapp_state1)?;

        assert_eq!(store.get_zkapp_state_num(&token, &pk)?.unwrap(), 2);

        // get zkapp states
        assert_eq!(
            store.get_zkapp_state(&token, &pk, 0)?.unwrap(),
            zkapp_state0
        );
        assert_eq!(
            store.get_zkapp_state(&token, &pk, 1)?.unwrap(),
            zkapp_state1
        );

        // remove last zkapp states
        store.remove_last_zkapp_state(&token, &pk)?;

        assert_eq!(store.get_zkapp_state_num(&token, &pk)?.unwrap(), 1);
        assert_eq!(
            store.get_zkapp_state(&token, &pk, 0)?.unwrap(),
            zkapp_state0
        );

        store.remove_last_zkapp_state(&token, &pk)?;
        assert_eq!(store.get_zkapp_state_num(&token, &pk)?.unwrap(), 0);

        Ok(())
    }

    #[test]
    fn zkapp_permissions() -> anyhow::Result<()> {
        let g = &mut Gen::new(GEN_SIZE);
        let store = create_indexer_store()?;

        let pk = PublicKey::arbitrary(g);
        let token = TokenAddress::arbitrary(g);

        assert!(store.get_zkapp_permissions_num(&token, &pk)?.is_none());

        // add zkapp permissions
        let permissions0 = Permissions::arbitrary(g);
        store.add_zkapp_permissions(&token, &pk, &permissions0)?;

        assert_eq!(store.get_zkapp_permissions_num(&token, &pk)?.unwrap(), 1);

        // add another zkapp permissions
        let permissions1 = Permissions::arbitrary(g);
        store.add_zkapp_permissions(&token, &pk, &permissions1)?;

        assert_eq!(store.get_zkapp_permissions_num(&token, &pk)?.unwrap(), 2);

        // get zkapp permissions
        assert_eq!(
            store.get_zkapp_permissions(&token, &pk, 0)?.unwrap(),
            permissions0
        );
        assert_eq!(
            store.get_zkapp_permissions(&token, &pk, 1)?.unwrap(),
            permissions1
        );

        // remove last zkapp permissions
        store.remove_last_zkapp_permissions(&token, &pk)?;

        assert_eq!(store.get_zkapp_permissions_num(&token, &pk)?.unwrap(), 1);
        assert_eq!(
            store.get_zkapp_permissions(&token, &pk, 0)?.unwrap(),
            permissions0
        );

        store.remove_last_zkapp_permissions(&token, &pk)?;
        assert_eq!(store.get_zkapp_permissions_num(&token, &pk)?.unwrap(), 0);

        Ok(())
    }

    #[test]
    fn zkapp_verification_key() -> anyhow::Result<()> {
        let g = &mut Gen::new(GEN_SIZE);
        let store = create_indexer_store()?;

        let pk = PublicKey::arbitrary(g);
        let token = TokenAddress::arbitrary(g);

        assert!(store.get_zkapp_verification_key_num(&token, &pk)?.is_none());

        // add zkapp verification key
        let verification_key0 = VerificationKey::arbitrary(g);
        store.add_zkapp_verification_key(&token, &pk, &verification_key0)?;

        assert_eq!(
            store.get_zkapp_verification_key_num(&token, &pk)?.unwrap(),
            1
        );

        // add another zkapp verification key
        let verification_key1 = VerificationKey::arbitrary(g);
        store.add_zkapp_verification_key(&token, &pk, &verification_key1)?;

        assert_eq!(
            store.get_zkapp_verification_key_num(&token, &pk)?.unwrap(),
            2
        );

        // get zkapp verification keys
        assert_eq!(
            store.get_zkapp_verification_key(&token, &pk, 0)?.unwrap(),
            verification_key0
        );
        assert_eq!(
            store.get_zkapp_verification_key(&token, &pk, 1)?.unwrap(),
            verification_key1
        );

        // remove last zkapp verification keys
        store.remove_last_zkapp_verification_key(&token, &pk)?;

        assert_eq!(
            store.get_zkapp_verification_key_num(&token, &pk)?.unwrap(),
            1
        );
        assert_eq!(
            store.get_zkapp_verification_key(&token, &pk, 0)?.unwrap(),
            verification_key0
        );

        store.remove_last_zkapp_verification_key(&token, &pk)?;
        assert_eq!(
            store.get_zkapp_verification_key_num(&token, &pk)?.unwrap(),
            0
        );

        Ok(())
    }

    #[test]
    fn zkapp_uri() -> anyhow::Result<()> {
        let g = &mut Gen::new(GEN_SIZE);
        let store = create_indexer_store()?;

        let pk = PublicKey::arbitrary(g);
        let token = TokenAddress::arbitrary(g);

        assert!(store.get_zkapp_uri_num(&token, &pk)?.is_none());

        // add zkapp uri
        let zkapp_uri0 = ZkappUri::arbitrary(g);
        store.add_zkapp_uri(&token, &pk, &zkapp_uri0)?;

        assert_eq!(store.get_zkapp_uri_num(&token, &pk)?.unwrap(), 1);

        // add another zkapp uri
        let zkapp_uri1 = ZkappUri::arbitrary(g);
        store.add_zkapp_uri(&token, &pk, &zkapp_uri1)?;

        assert_eq!(store.get_zkapp_uri_num(&token, &pk)?.unwrap(), 2);

        // get zkapp uris
        assert_eq!(store.get_zkapp_uri(&token, &pk, 0)?.unwrap(), zkapp_uri0);
        assert_eq!(store.get_zkapp_uri(&token, &pk, 1)?.unwrap(), zkapp_uri1);

        // remove last zkapp uris
        store.remove_last_zkapp_uri(&token, &pk)?;

        assert_eq!(store.get_zkapp_uri_num(&token, &pk)?.unwrap(), 1);
        assert_eq!(store.get_zkapp_uri(&token, &pk, 0)?.unwrap(), zkapp_uri0);

        store.remove_last_zkapp_uri(&token, &pk)?;
        assert_eq!(store.get_zkapp_uri_num(&token, &pk)?.unwrap(), 0);

        Ok(())
    }

    #[test]
    fn zkapp_token_symbol() -> anyhow::Result<()> {
        let g = &mut Gen::new(GEN_SIZE);
        let store = create_indexer_store()?;

        let pk = PublicKey::arbitrary(g);
        let token = TokenAddress::arbitrary(g);

        assert!(store.get_zkapp_token_symbol_num(&token, &pk)?.is_none());

        // add zkapp token symbol
        let token_symbol0 = TokenSymbol::arbitrary(g);
        store.add_zkapp_token_symbol(&token, &pk, &token_symbol0)?;

        assert_eq!(store.get_zkapp_token_symbol_num(&token, &pk)?.unwrap(), 1);

        // add another zkapp token symbol
        let token_symbol1 = TokenSymbol::arbitrary(g);
        store.add_zkapp_token_symbol(&token, &pk, &token_symbol1)?;

        assert_eq!(store.get_zkapp_token_symbol_num(&token, &pk)?.unwrap(), 2);

        // get zkapp token symbols
        assert_eq!(
            store.get_zkapp_token_symbol(&token, &pk, 0)?.unwrap(),
            token_symbol0
        );
        assert_eq!(
            store.get_zkapp_token_symbol(&token, &pk, 1)?.unwrap(),
            token_symbol1
        );

        // remove last zkapp token symbols
        store.remove_last_zkapp_token_symbol(&token, &pk)?;

        assert_eq!(store.get_zkapp_token_symbol_num(&token, &pk)?.unwrap(), 1);
        assert_eq!(
            store.get_zkapp_token_symbol(&token, &pk, 0)?.unwrap(),
            token_symbol0
        );

        store.remove_last_zkapp_token_symbol(&token, &pk)?;
        assert_eq!(store.get_zkapp_token_symbol_num(&token, &pk)?.unwrap(), 0);

        Ok(())
    }

    #[test]
    fn zkapp_timing() -> anyhow::Result<()> {
        let g = &mut Gen::new(GEN_SIZE);
        let store = create_indexer_store()?;

        let pk = PublicKey::arbitrary(g);
        let token = TokenAddress::arbitrary(g);

        assert!(store.get_zkapp_timing_num(&token, &pk)?.is_none());

        // add zkapp timing
        let timing0 = Timing::arbitrary(g);
        store.add_zkapp_timing(&token, &pk, &timing0)?;

        assert_eq!(store.get_zkapp_timing_num(&token, &pk)?.unwrap(), 1);

        // add another zkapp timing
        let timing1 = Timing::arbitrary(g);
        store.add_zkapp_timing(&token, &pk, &timing1)?;

        assert_eq!(store.get_zkapp_timing_num(&token, &pk)?.unwrap(), 2);

        // get zkapp timings
        assert_eq!(store.get_zkapp_timing(&token, &pk, 0)?.unwrap(), timing0);
        assert_eq!(store.get_zkapp_timing(&token, &pk, 1)?.unwrap(), timing1);

        // remove last zkapp timings
        store.remove_last_zkapp_timing(&token, &pk)?;

        assert_eq!(store.get_zkapp_timing_num(&token, &pk)?.unwrap(), 1);
        assert_eq!(store.get_zkapp_timing(&token, &pk, 0)?.unwrap(), timing0);

        store.remove_last_zkapp_timing(&token, &pk)?;
        assert_eq!(store.get_zkapp_timing_num(&token, &pk)?.unwrap(), 0);

        Ok(())
    }
}
