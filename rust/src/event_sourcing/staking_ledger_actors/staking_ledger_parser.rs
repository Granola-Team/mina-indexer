use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct StakingLedgerParserActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
}

impl StakingLedgerParserActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "StakingLedgerParserActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Actor for StakingLedgerParserActor {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn actor_outputs(&self) -> &AtomicUsize {
        &self.events_published
    }
    async fn handle_event(&self, event: Event) {
        if let EventType::StakingLedgerFilePath = event.event_type {
            println!("Parsed a staking ledger file path event")
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}
