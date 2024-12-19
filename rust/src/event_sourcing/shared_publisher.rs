use super::events::Event;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::broadcast;

pub struct SharedPublisher {
    sender: broadcast::Sender<Event>,
    database_inserts: AtomicUsize,
}

impl SharedPublisher {
    pub fn new(buffer_size: usize) -> Self {
        let (sender, _) = broadcast::channel(buffer_size);
        Self {
            sender,
            database_inserts: AtomicUsize::new(0),
        }
    }

    pub fn publish(&self, event: Event) {
        self.sender.send(event).unwrap();
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    pub fn incr_database_insert(&self) {
        self.database_inserts.fetch_add(1, Ordering::SeqCst);
    }
}
