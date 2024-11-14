use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::stream::payloads::{InternalCommandPayload, InternalCommandType, MainnetBlockPayload};
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct FeeTransferActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
}

impl FeeTransferActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "FeeTransferActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Actor for FeeTransferActor {
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
                for fee_transfer in block_payload.fee_transfers.iter() {
                    let payload = InternalCommandPayload {
                        internal_command_type: InternalCommandType::FeeTransfer,
                        height: block_payload.height,
                        state_hash: block_payload.state_hash.clone(),
                        timestamp: block_payload.timestamp,
                        recipient: fee_transfer.recipient.to_string(),
                        amount_nanomina: fee_transfer.fee_nanomina,
                    };
                    self.publish(Event {
                        event_type: EventType::InternalCommand,
                        payload: sonic_rs::to_string(&payload).unwrap(),
                    });
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
async fn test_fee_transfer_actor_handle_event() {
    use super::*;
    use crate::stream::{
        events::{Event, EventType},
        mainnet_block_models::*,
        payloads::{InternalCommandPayload, MainnetBlockPayload},
    };
    use std::sync::Arc;

    // Create a shared publisher to capture published events
    let shared_publisher = Arc::new(SharedPublisher::new(100));
    let actor = FeeTransferActor::new(Arc::clone(&shared_publisher));

    // Mock a MainnetBlockPayload with fee transfers
    let fee_transfers = vec![
        FeeTransfer {
            recipient: "recipient_1".to_string(),
            fee_nanomina: 100_000,
        },
        FeeTransfer {
            recipient: "recipient_2".to_string(),
            fee_nanomina: 200_000,
        },
    ];

    // MainnetBlockPayload with sample fee transfers
    let block_payload = MainnetBlockPayload {
        height: 15,
        state_hash: "state_hash_example".to_string(),
        previous_state_hash: "previous_state_hash_example".to_string(),
        last_vrf_output: "last_vrf_output_example".to_string(),
        fee_transfers,
        timestamp: 1615986540000,
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

    // Capture and verify the published InternalCommand events for fee transfers
    for fee_transfer in block_payload.fee_transfers.iter() {
        // Check if the InternalCommand event was published
        if let Ok(received_event) = receiver.recv().await {
            assert_eq!(received_event.event_type, EventType::InternalCommand);

            // Deserialize the payload of the InternalCommand event
            let command_payload: InternalCommandPayload = sonic_rs::from_str(&received_event.payload).unwrap();

            // Verify that the InternalCommandPayload matches the expected values
            assert_eq!(command_payload.internal_command_type, InternalCommandType::FeeTransfer);
            assert_eq!(command_payload.height, block_payload.height);
            assert_eq!(command_payload.state_hash, block_payload.state_hash);
            assert_eq!(command_payload.timestamp, block_payload.timestamp);
            assert_eq!(command_payload.recipient, fee_transfer.recipient);
            assert_eq!(command_payload.amount_nanomina, fee_transfer.fee_nanomina);
        } else {
            panic!("Did not receive expected InternalCommand event from FeeTransferActor.");
        }
    }

    // Verify that the event count matches the number of fee transfers processed
    assert_eq!(actor.actor_outputs().load(Ordering::SeqCst), block_payload.fee_transfers.len());
}
