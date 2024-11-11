use super::events::Event;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::broadcast;

pub struct SharedPublisher {
    sender: broadcast::Sender<Event>,
    buffer_count: AtomicUsize, // Tracks the current buffer usage
}

impl SharedPublisher {
    pub fn new(buffer_size: usize) -> Self {
        let (sender, _) = broadcast::channel(buffer_size);
        Self {
            sender,
            buffer_count: AtomicUsize::new(0),
        }
    }

    pub fn publish(&self, event: Event) {
        match self.sender.send(event) {
            Ok(_) => {
                self.buffer_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            Err(e) => {
                println!("{:?}", e);
            }
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    // Call this method to monitor the current buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer_count.load(Ordering::SeqCst)
    }

    // Method to decrement buffer count, call this in your consumer to track consumed messages
    pub fn message_consumed(&self) {
        self.buffer_count.fetch_sub(1, Ordering::SeqCst);
    }
}
