use super::events::Event;
use tokio::sync::broadcast;

pub struct SharedPublisher {
    sender: broadcast::Sender<Event>,
}

impl SharedPublisher {
    pub fn new(buffer_size: usize) -> Self {
        let (sender, _) = broadcast::channel(buffer_size);
        Self { sender }
    }

    pub fn publish(&self, event: Event) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }
}
