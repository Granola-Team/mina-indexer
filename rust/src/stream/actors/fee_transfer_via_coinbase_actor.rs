use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::stream::payloads::{InternalCommandLogPayload, InternalCommandType, MainnetBlockPayload};
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct FeeTransferViaCoinbaseActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
}

impl FeeTransferViaCoinbaseActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "FeeTransferViaCoinbaseActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Actor for FeeTransferViaCoinbaseActor {
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
                if let Some(fee_transfers_via_coinbase) = block_payload.fee_transfer_via_coinbase {
                    for fee_transfer_via_coinbase in fee_transfers_via_coinbase.iter() {
                        let payload = InternalCommandLogPayload {
                            internal_command_type: InternalCommandType::FeeTransferViaCoinbase,
                            height: block_payload.height,
                            state_hash: block_payload.state_hash.to_string(),
                            timestamp: block_payload.timestamp,
                            recipient: fee_transfer_via_coinbase.receiver.to_string(),
                            amount_nanomina: (fee_transfer_via_coinbase.fee * 1_000_000_000f64) as u64,
                            source: Some(block_payload.coinbase_receiver.to_string()),
                        };
                        self.publish(Event {
                            event_type: EventType::InternalCommandLog,
                            payload: sonic_rs::to_string(&payload).unwrap(),
                        });
                    }
                }
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

#[tokio::test]
async fn test_handle_mainnet_block_event_publishes_fee_transfer_via_coinbase_event() {
    use super::*;
    use crate::stream::{
        events::{Event, EventType},
        mainnet_block_models::FeeTransferViaCoinbase,
        payloads::MainnetBlockPayload,
    };
    use std::sync::Arc;

    // Setup a shared publisher to capture published events
    let shared_publisher = Arc::new(SharedPublisher::new(100));
    let actor = FeeTransferViaCoinbaseActor::new(Arc::clone(&shared_publisher));

    // Create a MainnetBlockPayload with a FeeTransferViaCoinbase
    let block_payload = MainnetBlockPayload {
        height: 10,
        state_hash: "state_hash_example".to_string(),
        timestamp: 123456789,
        fee_transfer_via_coinbase: Some(vec![FeeTransferViaCoinbase {
            receiver: "receiver_example".to_string(),
            fee: 0.00005,
        }]),
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

    // Capture and verify the published FeeTransferViaCoinbase event
    if let Ok(received_event) = receiver.recv().await {
        assert_eq!(received_event.event_type, EventType::InternalCommandLog);

        // Deserialize the payload of the FeeTransferViaCoinbase event
        let fee_transfer_payload: InternalCommandLogPayload = sonic_rs::from_str(&received_event.payload).unwrap();

        // Verify that the FeeTransferViaCoinbasePayload matches the expected values
        assert_eq!(fee_transfer_payload.height, block_payload.height);
        assert_eq!(fee_transfer_payload.state_hash, block_payload.state_hash);
        assert_eq!(fee_transfer_payload.timestamp, block_payload.timestamp);
        assert_eq!(fee_transfer_payload.recipient, "receiver_example");
        assert_eq!(fee_transfer_payload.amount_nanomina, 50_000); // 0.00005 * 1_000_000_000
    } else {
        panic!("Did not receive expected FeeTransferViaCoinbase event from FeeTransferViaCoinbaseActor.");
    }

    // Verify that the event count matches the number of events published
    assert_eq!(actor.actor_outputs().load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_handle_mainnet_block_event_without_fee_transfer_via_coinbase() {
    use super::*;
    use crate::stream::{
        events::{Event, EventType},
        payloads::MainnetBlockPayload,
    };
    use std::sync::Arc;

    // Setup a shared publisher to capture published events
    let shared_publisher = Arc::new(SharedPublisher::new(100));
    let actor = FeeTransferViaCoinbaseActor::new(Arc::clone(&shared_publisher));

    // Create a MainnetBlockPayload without a FeeTransferViaCoinbase
    let block_payload = MainnetBlockPayload {
        height: 10,
        state_hash: "state_hash_example".to_string(),
        timestamp: 123456789,
        fee_transfer_via_coinbase: None,
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

    // Verify that no events were published
    assert!(receiver.try_recv().is_err());
    assert_eq!(actor.actor_outputs().load(Ordering::SeqCst), 0);
}
