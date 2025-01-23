//! Zkapp event store trait

use crate::{
    ledger::{public_key::PublicKey, token::TokenAddress},
    mina_blocks::v2::ZkappEvent,
    store::Result,
};

pub trait ZkappEventStore {
    /// Add events to the token account
    ///
    /// Returns the total number of events for the account
    fn add_events(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        events: &[ZkappEvent],
    ) -> Result<u32>;

    /// Get the `index`th event for the token account
    fn get_event(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        index: u32,
    ) -> Result<Option<ZkappEvent>>;

    /// Set the `index`th event for the token account
    fn set_event(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        event: &ZkappEvent,
        index: u32,
    ) -> Result<()>;

    /// Get the total number of events associated with the token account
    fn get_num_events(&self, pk: &PublicKey, token: &TokenAddress) -> Result<Option<u32>>;

    /// Remove the specified number of events from the account
    ///
    /// Returns the number of remaining events
    fn remove_events(&self, pk: &PublicKey, token: &TokenAddress, num: u32) -> Result<u32>;

    /// Remove the event at the specified index from the account
    fn remove_event(&self, pk: &PublicKey, token: &TokenAddress, index: u32) -> Result<()>;
}
