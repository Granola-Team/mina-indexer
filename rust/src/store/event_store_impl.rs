use super::{column_families::ColumnFamilyHelpers, fixed_keys::FixedKeys};
use crate::{
    event::{
        db::{DbBlockEvent, DbEvent},
        store::EventStore,
        witness_tree::WitnessTreeEvent,
        IndexerEvent,
    },
    store::IndexerStore,
    utility::store::from_be_bytes,
};
use log::trace;

impl EventStore for IndexerStore {
    fn add_event(&self, event: &IndexerEvent) -> anyhow::Result<u32> {
        let seq_num = self.get_next_seq_num()?;
        trace!("Adding event {seq_num}: {event:?}");

        if matches!(
            event,
            IndexerEvent::WitnessTree(WitnessTreeEvent::UpdateBestTip { .. })
        ) {
            return Ok(seq_num);
        }

        // add prefixed event to db
        let mut value = match event {
            IndexerEvent::Db(DbEvent::Block(
                DbBlockEvent::NewBestTip {
                    blockchain_length, ..
                }
                | DbBlockEvent::NewBlock {
                    blockchain_length, ..
                },
            )) => (*blockchain_length).to_be_bytes().to_vec(),
            _ => 0u32.to_be_bytes().to_vec(),
        };
        value.push(event.kind());
        value.append(&mut serde_json::to_vec(&event)?);
        self.database
            .put_cf(self.events_cf(), seq_num.to_be_bytes(), value)?;

        // increment event sequence number
        let next_seq_num = seq_num + 1;
        self.database
            .put(Self::NEXT_EVENT_SEQ_NUM_KEY, next_seq_num.to_be_bytes())?;

        // return next event sequence number
        Ok(next_seq_num)
    }

    fn get_event(&self, seq_num: u32) -> anyhow::Result<Option<IndexerEvent>> {
        trace!("Getting event {seq_num}");
        Ok(self
            .database
            .get_pinned_cf(self.events_cf(), seq_num.to_be_bytes())?
            .and_then(|bytes| serde_json::from_slice(&bytes[5..]).ok()))
    }

    fn get_next_seq_num(&self) -> anyhow::Result<u32> {
        trace!("Getting next event sequence number");
        Ok(self
            .database
            .get(Self::NEXT_EVENT_SEQ_NUM_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn get_event_log(&self) -> anyhow::Result<Vec<IndexerEvent>> {
        trace!("Getting event log");
        let mut events = vec![];
        for n in 0..self.get_next_seq_num()? {
            if let Some(event) = self.get_event(n)? {
                events.push(event);
            }
        }
        Ok(events)
    }

    /// Key: sequence number ([u32] BE bytes)
    /// Value: event (serialized with [serde_json::to_vec])
    fn event_log_iterator(&self, mode: speedb::IteratorMode) -> speedb::DBIterator<'_> {
        self.database.iterator_cf(self.events_cf(), mode)
    }
}
