use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::stream::payloads::{InternalCommandPayload, InternalCommandType, MainnetBlockPayload};
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct CoinbaseTransferActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
}

impl CoinbaseTransferActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "CoinbaseTransferActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Actor for CoinbaseTransferActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.events_published
    }
    async fn handle_event(&self, event: Event) {
        match event.event_type {
            EventType::MainnetBlock => {
                let block_payload: MainnetBlockPayload = sonic_rs::from_str(&event.payload).unwrap();
                let payload = InternalCommandPayload {
                    internal_command_type: InternalCommandType::Coinbase,
                    height: block_payload.height,
                    state_hash: block_payload.state_hash.to_string(),
                    timestamp: block_payload.timestamp,
                    recipient: block_payload.coinbase_receiver,
                    amount_nanomina: block_payload.coinbase_reward_nanomina,
                    source: None,
                };
                self.publish(Event {
                    event_type: EventType::InternalCommand,
                    payload: sonic_rs::to_string(&payload).unwrap(),
                });
            }
            EventType::BerkeleyBlock => {
                todo!("impl for berkeley block");
            }
            _ => {}
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}
