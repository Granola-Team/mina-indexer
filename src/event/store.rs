use crate::event::Event;

pub trait EventStore {
    /// Add event to db and return the next sequence number
    fn add_event(&self, event: &Event) -> anyhow::Result<u32>;

    /// Get the event from the log
    fn get_event(&self, seq_num: u32) -> anyhow::Result<Option<Event>>;

    /// Get the next event sequence number
    fn get_next_seq_num(&self) -> anyhow::Result<u32>;

    /// Returns the event log
    fn get_event_log(&self) -> anyhow::Result<Vec<Event>>;
}
