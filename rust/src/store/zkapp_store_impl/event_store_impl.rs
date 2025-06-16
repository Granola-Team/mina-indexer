//! Zkapp event store impol

use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    command::TxnHash,
    ledger::token::TokenAddress,
    mina_blocks::v2::{zkapp::event::ZkappEventWithMeta, ZkappEvent},
    store::{
        column_families::ColumnFamilyHelpers, zkapp::events::ZkappEventStore, IndexerStore, Result,
    },
    utility::store::{
        common::{from_be_bytes, U32_LEN},
        zkapp::events::{zkapp_events_key, zkapp_events_pk_num_key},
    },
};
use anyhow::Context;
use log::trace;
use speedb::Direction;

impl ZkappEventStore for IndexerStore {
    fn add_events(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        events: &[ZkappEvent],
        state_hash: &StateHash,
        block_height: u32,
        txn_hash: &TxnHash,
    ) -> Result<u32> {
        trace!("Adding events to token account ({pk}, {token}): {events:?}");

        let idx = self.get_num_events(pk, token)?.unwrap_or_default();
        let mut num = idx;

        // add each event
        for event in events.iter().cloned() {
            let event = ZkappEventWithMeta {
                event,
                txn_hash: txn_hash.to_owned(),
                state_hash: state_hash.to_owned(),
                block_height,
            };

            self.set_event(pk, token, &event, num)?;
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
    ) -> Result<Option<ZkappEventWithMeta>> {
        trace!("Getting event {index} for token account ({pk}, {token})");

        Ok(self
            .database
            .get_cf(self.zkapp_events_cf(), zkapp_events_key(token, pk, index))?
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
        event: &ZkappEventWithMeta,
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

    ///////////////
    // Iterators //
    ///////////////

    fn events_iterator(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        index: Option<u32>,
        direction: Direction,
    ) -> speedb::DBIterator<'_> {
        const LEN: usize = TokenAddress::LEN + PublicKey::LEN + U32_LEN;

        let mut start = [0; LEN + 1];
        match direction {
            Direction::Forward => {
                let index = index.unwrap_or_default();
                start[..LEN].copy_from_slice(&zkapp_events_key(token, pk, index))
            }
            Direction::Reverse => {
                let index = index.unwrap_or(u32::MAX);
                start[..LEN].copy_from_slice(&zkapp_events_key(token, pk, index));

                if index == u32::MAX {
                    start[LEN] = 1;
                }
            }
        };

        self.database.iterator_cf(
            self.zkapp_events_cf(),
            speedb::IteratorMode::From(&start, direction),
        )
    }
}
