use crate::event::IndexerEvent;

pub trait EventStore {
    /// Add event to db and return the next sequence number
    ///
    /// `WitnessTree` events are not recorded
    fn add_event(&self, event: &IndexerEvent) -> anyhow::Result<u32>;

    /// Get the event from the log
    fn get_event(&self, seq_num: u32) -> anyhow::Result<Option<IndexerEvent>>;

    /// Get the next event sequence number
    fn get_next_seq_num(&self) -> anyhow::Result<u32>;

    /// Returns the event log
    fn get_event_log(&self) -> anyhow::Result<Vec<IndexerEvent>>;
}
