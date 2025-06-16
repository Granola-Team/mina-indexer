//! Zkapp event store trait

use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    command::TxnHash,
    ledger::token::TokenAddress,
    mina_blocks::v2::{zkapp::event::ZkappEventWithMeta, ZkappEvent},
    store::Result,
};
use speedb::{DBIterator, Direction};

pub trait ZkappEventStore {
    /// Add events to the token account
    ///
    /// Returns the total number of events for the account
    fn add_events(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        events: &[ZkappEvent],
        state_hash: &StateHash,
        block_height: u32,
        txn_hash: &TxnHash,
    ) -> Result<u32>;

    /// Get the `index`th event for the token account
    fn get_event(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        index: u32,
    ) -> Result<Option<ZkappEventWithMeta>>;

    /// Set the `index`th event for the token account
    fn set_event(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        event: &ZkappEventWithMeta,
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

    ///////////////
    // Iterators //
    ///////////////

    /// Iterator over pk/token events from start height (inclusive) to end
    /// height (exclusive)
    fn events_iterator(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        index: Option<u32>,
        direction: Direction,
    ) -> DBIterator<'_>;
}
