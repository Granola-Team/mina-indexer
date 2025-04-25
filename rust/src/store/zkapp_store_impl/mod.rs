//! Zkapp store trait implementation

use super::{
    zkapp::{ZkappState, ZkappStore},
    IndexerStore, Result,
};
use crate::{
    base::public_key::PublicKey,
    ledger::{
        account::{Permissions, Timing},
        token::{TokenAddress, TokenSymbol},
    },
    mina_blocks::v2::{VerificationKey, ZkappUri},
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
            (index - 1).to_be_bytes(),
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
            (index - 1).to_be_bytes(),
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
