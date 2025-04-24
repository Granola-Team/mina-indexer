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
};

pub mod action_store_impl;
pub mod event_store_impl;
pub mod token_store_impl;

impl ZkappStore for IndexerStore {
    ///////////////
    // app state //
    ///////////////

    fn get_zkapp_state_num(&self, _token: &TokenAddress, _pk: &PublicKey) -> Result<Option<u32>> {
        Ok(None)
    }

    fn get_zkapp_state(
        &self,
        _token: &TokenAddress,
        _pk: &PublicKey,
        _index: u32,
    ) -> Result<Option<ZkappState>> {
        Ok(None)
    }

    fn add_zkapp_state(
        &self,
        _token: &TokenAddress,
        _pk: &PublicKey,
        _app_state: &ZkappState,
    ) -> Result<()> {
        Ok(())
    }

    fn remove_last_zkapp_state(
        &self,
        _token: &TokenAddress,
        _pk: &PublicKey,
    ) -> Result<ZkappState> {
        Ok(Default::default())
    }

    /////////////////
    // permissions //
    /////////////////

    fn get_zkapp_permissions_num(
        &self,
        _token: &TokenAddress,
        _pk: &PublicKey,
    ) -> Result<Option<u32>> {
        Ok(None)
    }

    fn get_zkapp_permissions(
        &self,
        _token: &TokenAddress,
        _pk: &PublicKey,
        _index: u32,
    ) -> Result<Option<Permissions>> {
        Ok(None)
    }

    fn add_zkapp_permissions(
        &self,
        _token: &TokenAddress,
        _pk: &PublicKey,
        _permissions: &Permissions,
    ) -> Result<()> {
        Ok(())
    }

    fn remove_last_zkapp_permissions(
        &self,
        _token: &TokenAddress,
        _pk: &PublicKey,
    ) -> Result<Permissions> {
        Ok(Default::default())
    }

    //////////////////////
    // verification key //
    //////////////////////

    fn get_zkapp_verification_key_num(
        &self,
        _token: &TokenAddress,
        _pk: &PublicKey,
    ) -> Result<Option<u32>> {
        Ok(None)
    }

    fn get_zkapp_verification_key(
        &self,
        _token: &TokenAddress,
        _pk: &PublicKey,
        _index: u32,
    ) -> Result<Option<VerificationKey>> {
        Ok(None)
    }

    fn add_zkapp_verification_key(
        &self,
        _token: &TokenAddress,
        _pk: &PublicKey,
        _verification_key: &VerificationKey,
    ) -> Result<()> {
        Ok(())
    }

    fn remove_last_zkapp_verification_key(
        &self,
        _token: &TokenAddress,
        _pk: &PublicKey,
    ) -> Result<VerificationKey> {
        Ok(Default::default())
    }

    ///////////////
    // zkapp uri //
    ///////////////

    fn get_zkapp_uri_num(&self, _token: &TokenAddress, _pk: &PublicKey) -> Result<Option<u32>> {
        Ok(None)
    }

    fn get_zkapp_uri(
        &self,
        _token: &TokenAddress,
        _pk: &PublicKey,
        _index: u32,
    ) -> Result<Option<ZkappUri>> {
        Ok(None)
    }

    fn add_zkapp_uri(
        &self,
        _token: &TokenAddress,
        _pk: &PublicKey,
        _zkapp_uri: &ZkappUri,
    ) -> Result<()> {
        Ok(())
    }

    fn remove_last_zkapp_uri(&self, _token: &TokenAddress, _pk: &PublicKey) -> Result<ZkappUri> {
        Ok(Default::default())
    }

    //////////////////
    // token symbol //
    //////////////////

    fn get_zkapp_token_symbol_num(
        &self,
        _token: &TokenAddress,
        _pk: &PublicKey,
    ) -> Result<Option<u32>> {
        Ok(None)
    }

    fn get_zkapp_token_symbol(
        &self,
        _token: &TokenAddress,
        _pk: &PublicKey,
        _index: u32,
    ) -> Result<Option<TokenSymbol>> {
        Ok(None)
    }

    fn add_zkapp_token_symbol(
        &self,
        _token: &TokenAddress,
        _pk: &PublicKey,
        _token_symbol: &TokenSymbol,
    ) -> Result<()> {
        Ok(())
    }

    fn remove_last_zkapp_token_symbol(
        &self,
        _token: &TokenAddress,
        _pk: &PublicKey,
    ) -> Result<TokenSymbol> {
        Ok(Default::default())
    }

    ////////////
    // timing //
    ////////////

    fn get_zkapp_timing_num(&self, _token: &TokenAddress, _pk: &PublicKey) -> Result<Option<u32>> {
        Ok(None)
    }

    fn get_zkapp_timing(
        &self,
        _token: &TokenAddress,
        _pk: &PublicKey,
        _index: u32,
    ) -> Result<Option<Timing>> {
        Ok(None)
    }

    fn add_zkapp_timing(
        &self,
        _token: &TokenAddress,
        _pk: &PublicKey,
        _timing: &Timing,
    ) -> Result<()> {
        Ok(())
    }

    fn remove_last_zkapp_timing(&self, _token: &TokenAddress, _pk: &PublicKey) -> Result<Timing> {
        Ok(Default::default())
    }
}
