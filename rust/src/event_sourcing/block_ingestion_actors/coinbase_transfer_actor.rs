use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::event_sourcing::payloads::{BerkeleyBlockPayload, InternalCommandLogPayload, InternalCommandType, MainnetBlockPayload};
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
                let payload = InternalCommandLogPayload {
                    internal_command_type: InternalCommandType::Coinbase,
                    height: block_payload.height,
                    state_hash: block_payload.state_hash.to_string(),
                    timestamp: block_payload.timestamp,
                    recipient: block_payload.coinbase_receiver,
                    amount_nanomina: block_payload.coinbase_reward_nanomina,
                    source: None,
                };
                self.publish(Event {
                    event_type: EventType::InternalCommandLog,
                    payload: sonic_rs::to_string(&payload).unwrap(),
                });
            }
            EventType::BerkeleyBlock => {
                let block_payload: BerkeleyBlockPayload = sonic_rs::from_str(&event.payload).unwrap();
                let payload = InternalCommandLogPayload {
                    internal_command_type: InternalCommandType::Coinbase,
                    height: block_payload.height,
                    state_hash: block_payload.state_hash.to_string(),
                    timestamp: block_payload.timestamp,
                    recipient: block_payload.coinbase_receiver,
                    amount_nanomina: block_payload.coinbase_reward_nanomina,
                    source: None,
                };
                self.publish(Event {
                    event_type: EventType::InternalCommandLog,
                    payload: sonic_rs::to_string(&payload).unwrap(),
                });
            }
            _ => {}
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[cfg(test)]
mod coinbase_transfer_actor_tests {

    use super::*;
    use crate::event_sourcing::{
        events::{Event, EventType},
        payloads::MainnetBlockPayload,
    };
    use std::sync::{atomic::Ordering, Arc};

    #[tokio::test]
    async fn test_handle_mainnet_block_event_publishes_coinbase_transfer_event() {
        // Setup a shared publisher to capture published events
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = CoinbaseTransferActor::new(Arc::clone(&shared_publisher));

        // Create a MainnetBlockPayload with a coinbase transfer
        let block_payload = MainnetBlockPayload {
            height: 10,
            state_hash: "state_hash_example".to_string(),
            timestamp: 123456789,
            coinbase_receiver: "receiver_example".to_string(),
            coinbase_reward_nanomina: 720_000_000,
            ..Default::default()
        };

        // Serialize the MainnetBlockPayload to JSON for the event payload
        let payload_json = sonic_rs::to_string(&block_payload).unwrap();
        let event = Event {
            event_type: EventType::MainnetBlock,
            payload: payload_json,
        };

        // Subscribe to the shared publisher to capture published events
        let mut receiver = shared_publisher.subscribe();

        // Call handle_event to process the MainnetBlock event
        actor.handle_event(event).await;

        // Capture and verify the published Coinbase transfer event
        if let Ok(received_event) = receiver.recv().await {
            assert_eq!(received_event.event_type, EventType::InternalCommandLog);

            // Deserialize the payload of the Coinbase transfer event
            let coinbase_payload: InternalCommandLogPayload = sonic_rs::from_str(&received_event.payload).unwrap();

            // Verify that the payload matches the expected values
            assert_eq!(coinbase_payload.height, block_payload.height);
            assert_eq!(coinbase_payload.state_hash, block_payload.state_hash);
            assert_eq!(coinbase_payload.timestamp, block_payload.timestamp);
            assert_eq!(coinbase_payload.recipient, block_payload.coinbase_receiver);
            assert_eq!(coinbase_payload.amount_nanomina, block_payload.coinbase_reward_nanomina);
        } else {
            panic!("Did not receive expected Coinbase transfer event from CoinbaseTransferActor.");
        }

        // Verify that the event count matches the number of coinbase transfers processed
        assert_eq!(actor.actor_outputs().load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_handle_berkeley_block_event_publishes_coinbase_transfer_event() {
        // Setup a shared publisher to capture published events
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = CoinbaseTransferActor::new(Arc::clone(&shared_publisher));

        // Create a BerkeleyBlockPayload with a coinbase transfer
        let block_payload = BerkeleyBlockPayload {
            height: 20,
            state_hash: "berkeley_state_hash_example".to_string(),
            timestamp: 987654321,
            coinbase_receiver: "berkeley_receiver_example".to_string(),
            coinbase_reward_nanomina: 360_000_000,
            ..Default::default()
        };

        // Serialize the BerkeleyBlockPayload to JSON for the event payload
        let payload_json = sonic_rs::to_string(&block_payload).unwrap();
        let event = Event {
            event_type: EventType::BerkeleyBlock,
            payload: payload_json,
        };

        // Subscribe to the shared publisher to capture published events
        let mut receiver = shared_publisher.subscribe();

        // Call handle_event to process the BerkeleyBlock event
        actor.handle_event(event).await;

        // Capture and verify the published Coinbase transfer event
        if let Ok(received_event) = receiver.recv().await {
            assert_eq!(received_event.event_type, EventType::InternalCommandLog);

            // Deserialize the payload of the Coinbase transfer event
            let coinbase_payload: InternalCommandLogPayload = sonic_rs::from_str(&received_event.payload).unwrap();

            // Verify that the payload matches the expected values
            assert_eq!(coinbase_payload.height, block_payload.height);
            assert_eq!(coinbase_payload.state_hash, block_payload.state_hash);
            assert_eq!(coinbase_payload.timestamp, block_payload.timestamp);
            assert_eq!(coinbase_payload.recipient, block_payload.coinbase_receiver);
            assert_eq!(coinbase_payload.amount_nanomina, block_payload.coinbase_reward_nanomina);
        } else {
            panic!("Did not receive expected Coinbase transfer event from CoinbaseTransferActor.");
        }

        // Verify that the event count matches the number of coinbase transfers processed
        assert_eq!(actor.actor_outputs().load(Ordering::SeqCst), 1);
    }
}
