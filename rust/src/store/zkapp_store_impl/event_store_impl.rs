//! Zkapp event store impol

use crate::{
    base::public_key::PublicKey,
    ledger::token::TokenAddress,
    mina_blocks::v2::ZkappEvent,
    store::{
        column_families::ColumnFamilyHelpers, zkapp::events::ZkappEventStore, IndexerStore, Result,
    },
    utility::store::{
        common::from_be_bytes,
        zkapp::events::{zkapp_events_key, zkapp_events_pk_num_key},
    },
};
use anyhow::Context;
use log::trace;

impl ZkappEventStore for IndexerStore {
    fn add_events(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        events: &[ZkappEvent],
    ) -> Result<u32> {
        trace!("Adding events to token account ({pk}, {token}): {events:?}");

        let idx = self.get_num_events(pk, token)?.unwrap_or_default();
        let mut num = idx;

        // add each event
        for event in events.iter() {
            self.set_event(pk, token, event, num)?;
            num += 1;
        }

        // update number of events
        self.database.put_cf(
            self.zkapp_events_pk_num_cf(),
            zkapp_events_pk_num_key(token, pk),
            num.to_be_bytes(),
        )?;

        Ok(num)
    }

    fn get_event(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        index: u32,
    ) -> Result<Option<ZkappEvent>> {
        trace!("Getting event {index} for token account ({pk}, {token})");

        Ok(self
            .database
            .get_pinned_cf(self.zkapp_events_cf(), zkapp_events_key(token, pk, index))?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .context(format!("missing {index} event for ({pk}, {token})"))
                    .unwrap()
            }))
    }

    fn set_event(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        event: &ZkappEvent,
        index: u32,
    ) -> Result<()> {
        trace!("Setting event {index} for token account ({pk}, {token})");

        Ok(self.database.put_cf(
            self.zkapp_events_cf(),
            zkapp_events_key(token, pk, index),
            serde_json::to_vec(event)?,
        )?)
    }

    fn get_num_events(&self, pk: &PublicKey, token: &TokenAddress) -> Result<Option<u32>> {
        trace!("Getting number of events for token account ({pk}, {token})");

        Ok(self
            .database
            .get_cf(
                self.zkapp_events_pk_num_cf(),
                zkapp_events_pk_num_key(token, pk),
            )?
            .map(from_be_bytes))
    }

    fn remove_events(&self, pk: &PublicKey, token: &TokenAddress, n: u32) -> Result<u32> {
        trace!("Removing {n} events from token account ({pk}, {token})");

        let mut num = self.get_num_events(pk, token)?.unwrap_or_default();
        assert!(n <= num);

        // remove each event
        for _ in 0..n {
            num -= 1;
            self.remove_event(pk, token, num)?;
        }

        // update number of events
        self.database.put_cf(
            self.zkapp_events_pk_num_cf(),
            zkapp_events_pk_num_key(token, pk),
            num.to_be_bytes(),
        )?;

        Ok(num)
    }

    fn remove_event(&self, pk: &PublicKey, token: &TokenAddress, index: u32) -> Result<()> {
        trace!("Removing {index}-th event from token account ({pk}, {token})");

        Ok(self
            .database
            .delete_cf(self.zkapp_events_cf(), zkapp_events_key(token, pk, index))?)
    }
}
