//! Zkapp store trait
//!
//! Defines the functionality for:
//! - zkapp accounts
//! - zkapp transactions
//! - minted tokens
//! - actions
//! - events

use crate::{
    base::public_key::PublicKey,
    ledger::{
        account::{Permissions, Timing},
        token::{TokenAddress, TokenSymbol},
    },
    mina_blocks::v2::{VerificationKey, ZkappState, ZkappUri},
    store::Result,
};

pub mod actions;
pub mod events;
pub mod tokens;

pub trait ZkappStore {
    ///////////////
    // app state //
    ///////////////

    /// Get the count of zkapp state changes
    fn get_zkapp_state_num(&self, token: &TokenAddress, pk: &PublicKey) -> Result<Option<u32>>;

    /// Get the zkapp state at the specified index
    fn get_zkapp_state(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        index: u32,
    ) -> Result<Option<ZkappState>>;

    /// Add zkapp state
    fn add_zkapp_state(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        app_state: &ZkappState,
    ) -> Result<()>;

    /// Remove the most recent zkapp state & return it
    ///
    /// Returns an error if no app state to remove
    fn remove_last_zkapp_state(&self, token: &TokenAddress, pk: &PublicKey) -> Result<ZkappState>;

    /////////////////
    // permissions //
    /////////////////

    /// Get the count of zkapp permissions changes
    fn get_zkapp_permissions_num(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
    ) -> Result<Option<u32>>;

    /// Get the zkapp permissions at the specified index
    fn get_zkapp_permissions(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        index: u32,
    ) -> Result<Option<Permissions>>;

    /// Add zkapp permissions
    fn add_zkapp_permissions(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        permissions: &Permissions,
    ) -> Result<()>;

    /// Remove the most recent zkapp permissions & return it
    ///
    /// Returns an error if no permissions to remove
    fn remove_last_zkapp_permissions(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
    ) -> Result<Permissions>;

    //////////////////////
    // verification key //
    //////////////////////

    /// Get the count of zkapp verification key changes
    fn get_zkapp_verification_key_num(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
    ) -> Result<Option<u32>>;

    /// Get the zkapp verification key at the specified index
    fn get_zkapp_verification_key(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        index: u32,
    ) -> Result<Option<VerificationKey>>;

    /// Add zkapp verification key
    fn add_zkapp_verification_key(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        verification_key: &VerificationKey,
    ) -> Result<()>;

    /// Remove the most recent zkapp verification key & return it
    ///
    /// Returns an error if no verification key to remove
    fn remove_last_zkapp_verification_key(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
    ) -> Result<VerificationKey>;

    ///////////////
    // zkapp uri //
    ///////////////

    /// Get the count of zkapp uri changes
    fn get_zkapp_uri_num(&self, token: &TokenAddress, pk: &PublicKey) -> Result<Option<u32>>;

    /// Get the zkapp uri at the specified index
    fn get_zkapp_uri(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        index: u32,
    ) -> Result<Option<ZkappUri>>;

    /// Add zkapp uri
    fn add_zkapp_uri(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        zkapp_uri: &ZkappUri,
    ) -> Result<()>;

    /// Remove the most recent zkapp uri & return it
    ///
    /// Returns an error if no zkapp uri to remove
    fn remove_last_zkapp_uri(&self, token: &TokenAddress, pk: &PublicKey) -> Result<ZkappUri>;

    //////////////////
    // token symbol //
    //////////////////

    /// Get the count of zkapp token symbol changes
    fn get_zkapp_token_symbol_num(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
    ) -> Result<Option<u32>>;

    /// Get the zkapp token symbol at the specified index
    fn get_zkapp_token_symbol(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        index: u32,
    ) -> Result<Option<TokenSymbol>>;

    /// Add zkapp token symbol
    fn add_zkapp_token_symbol(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        token_symbol: &TokenSymbol,
    ) -> Result<()>;

    /// Remove the most recent zkapp token symbol & return it
    ///
    /// Returns an error if no token symbol to remove
    fn remove_last_zkapp_token_symbol(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
    ) -> Result<TokenSymbol>;

    ////////////
    // timing //
    ////////////

    /// Get the count of zkapp timing changes
    fn get_zkapp_timing_num(&self, token: &TokenAddress, pk: &PublicKey) -> Result<Option<u32>>;

    /// Get the zkapp timing at the specified index
    fn get_zkapp_timing(
        &self,
        token: &TokenAddress,
        pk: &PublicKey,
        index: u32,
    ) -> Result<Option<Timing>>;

    /// Add zkapp timing
    fn add_zkapp_timing(&self, token: &TokenAddress, pk: &PublicKey, timing: &Timing)
        -> Result<()>;

    /// Remove the most recent zkapp timing & return it
    ///
    /// Returns an error if no timing to remove
    fn remove_last_zkapp_timing(&self, token: &TokenAddress, pk: &PublicKey) -> Result<Timing>;
}
