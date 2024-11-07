use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::stream::{
    models::{PreviousStateHash, StateHash},
    payloads::NewBlockAddedPayload,
};
use async_trait::async_trait;
use futures::lock::Mutex;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc},
};

pub struct BlockCanonicityActor {
    id: String,
    shared_publisher: Arc<SharedPublisher>,
    events_processed: AtomicUsize,
    blockchain_tree: Arc<Mutex<HashMap<StateHash, PreviousStateHash>>>,
}

impl BlockCanonicityActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "BlockCanonicityActor".to_string(),
            shared_publisher,
            events_processed: AtomicUsize::new(0),
            blockchain_tree: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Actor for BlockCanonicityActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn events_processed(&self) -> &AtomicUsize {
        &self.events_processed
    }
    async fn handle_event(&self, event: Event) {
        match event.event_type {
            EventType::BlockAddedToTree => {
                let payload: NewBlockAddedPayload = sonic_rs::from_str(&event.payload).unwrap();

                self.publish(Event {
                    event_type: EventType::BlockCanonicityUpdate,
                    payload: sonic_rs::to_string(&block_ancestor_payload).unwrap(),
                });
                // self.incr_event_processed();
            }
            _ => {}
        }
    }

    fn publish(&self, event: Event) {
        self.shared_publisher.publish(event);
    }
}
