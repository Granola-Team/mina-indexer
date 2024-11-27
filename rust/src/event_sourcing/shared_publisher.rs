use super::events::Event;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::broadcast;

pub struct SharedPublisher {
    sender: broadcast::Sender<Event>,
    high_priority_sender: broadcast::Sender<Event>,
    buffer_count: AtomicUsize,
    database_inserts: AtomicUsize,
}

impl SharedPublisher {
    pub fn new(buffer_size: usize) -> Self {
        let (sender, _) = broadcast::channel(buffer_size);
        let (high_priority_sender, _) = broadcast::channel(1_000);
        Self {
            sender,
            high_priority_sender,
            buffer_count: AtomicUsize::new(0),
            database_inserts: AtomicUsize::new(0),
        }
    }

    pub fn publish(&self, event: Event) {
        match self.sender.send(event) {
            Ok(_) => {
                self.buffer_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            Err(_) => {
                // println!("{:?}", e);
            }
        }
    }

    pub fn publish_high_priority(&self, event: Event) {
        match self.high_priority_sender.send(event) {
            Ok(_) => {
                self.buffer_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            Err(_) => {
                // println!("{:?}", e);
            }
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    pub fn subscribe_high_priority(&self) -> broadcast::Receiver<Event> {
        self.high_priority_sender.subscribe()
    }

    // Call this method to monitor the current buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer_count.load(Ordering::SeqCst)
    }

    pub fn database_inserts(&self) -> usize {
        self.database_inserts.load(Ordering::SeqCst)
    }

    pub fn incr_database_insert(&self) {
        self.database_inserts.fetch_add(1, Ordering::SeqCst);
    }

    // Method to decrement buffer count, call this in your consumer to track consumed messages
    pub fn message_consumed(&self) {
        self.buffer_count.fetch_sub(1, Ordering::SeqCst);
    }
}
