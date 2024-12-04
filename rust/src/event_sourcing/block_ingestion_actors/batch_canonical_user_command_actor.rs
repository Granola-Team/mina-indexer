use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    constants::TRANSITION_FRONTIER_DISTANCE,
    event_sourcing::{canonical_items_manager::CanonicalItemsManager, payloads::*},
};
use async_trait::async_trait;
use futures::lock::Mutex;
use log::error;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct BatchCanonicalUserCommandLogActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
    pub canonical_items_manager: Arc<Mutex<CanonicalItemsManager<BatchCanonicalUserCommandLogPayload>>>,
}

impl BatchCanonicalUserCommandLogActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "BatchCanonicalUserCommandLogActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
            canonical_items_manager: Arc::new(Mutex::new(CanonicalItemsManager::new((TRANSITION_FRONTIER_DISTANCE / 5usize) as u64))),
        }
    }
}

#[async_trait]
impl Actor for BatchCanonicalUserCommandLogActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.events_published
    }

    async fn report(&self) {
        let manager = self.canonical_items_manager.lock().await;
        manager.report(&self.id()).await;
    }

    async fn handle_event(&self, event: Event) {
        match event.event_type {
            EventType::BlockCanonicityUpdate => {
                let payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&event.payload).unwrap();
                {
                    let manager = self.canonical_items_manager.lock().await;
                    manager.add_block_canonicity_update(payload.clone()).await;
                }
                {
                    let manager = self.canonical_items_manager.lock().await;
                    for payload in manager.get_updates(payload.height).await.iter() {
                        self.publish(Event {
                            event_type: EventType::BatchCanonicalUserCommandLog,
                            payload: sonic_rs::to_string(&payload).unwrap(),
                        });
                    }

                    if let Err(e) = manager.prune().await {
                        error!("{}", e);
                    }
                }
            }
            EventType::MainnetBlock => {
                let event_payload: MainnetBlockPayload = sonic_rs::from_str(&event.payload).unwrap();
                {
                    let manager = self.canonical_items_manager.lock().await;
                    manager.add_items_count(event_payload.height, &event_payload.state_hash, 1).await;
                    let bulk_payload = BatchCanonicalUserCommandLogPayload {
                        height: event_payload.height,
                        state_hash: event_payload.state_hash.to_string(),
                        global_slot: event_payload.global_slot,
                        commands: event_payload.user_commands,
                        timestamp: event_payload.timestamp,
                        canonical: true,     // some default value
                        was_canonical: true, // some default value
                    };
                    manager.add_item(bulk_payload).await;
                }
                {
                    let manager = self.canonical_items_manager.lock().await;
                    for payload in manager.get_updates(event_payload.height).await.iter() {
                        self.publish(Event {
                            event_type: EventType::BatchCanonicalUserCommandLog,
                            payload: sonic_rs::to_string(&payload).unwrap(),
                        });
                    }

                    if let Err(e) = manager.prune().await {
                        error!("{}", e);
                    }
                }
            }
            _ => return,
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[cfg(test)]
mod batch_canonical_user_command_tests {
    use super::*;
    use crate::event_sourcing::{
        events::{Event, EventType},
        mainnet_block_models::{CommandStatus, CommandSummary, CommandType},
        payloads::{BatchCanonicalUserCommandLogPayload, BlockCanonicityUpdatePayload, MainnetBlockPayload},
    };
    use std::sync::{atomic::Ordering, Arc};

    #[tokio::test]
    async fn test_handle_event_block_canonicity_update_and_mainnet_block_with_user_commands() {
        // Setup shared publisher and actor
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = BatchCanonicalUserCommandLogActor::new(shared_publisher.clone());

        // Create mock user commands
        let user_commands = vec![
            CommandSummary {
                memo: "Test Memo 1".to_string(),
                fee_payer: "payer1".to_string(),
                sender: "sender1".to_string(),
                receiver: "receiver1".to_string(),
                status: CommandStatus::Applied,
                txn_type: CommandType::Payment,
                nonce: 1,
                fee_nanomina: 1000,
                amount_nanomina: 5000,
            },
            CommandSummary {
                memo: "Test Memo 2".to_string(),
                fee_payer: "payer2".to_string(),
                sender: "sender2".to_string(),
                receiver: "receiver2".to_string(),
                status: CommandStatus::Failed,
                txn_type: CommandType::StakeDelegation,
                nonce: 2,
                fee_nanomina: 2000,
                amount_nanomina: 10000,
            },
        ];

        // Create a mock MainnetBlockPayload
        let mainnet_payload = MainnetBlockPayload {
            height: 10,
            state_hash: "test_state_hash".to_string(),
            global_slot: 12345,
            user_commands: user_commands.clone(), // Include mock user commands
            ..Default::default()
        };

        // Create a mock BlockCanonicityUpdatePayload
        let canonicity_payload = BlockCanonicityUpdatePayload {
            height: 10,
            state_hash: "test_state_hash".to_string(),
            canonical: true,
            was_canonical: true,
        };

        // Serialize the payloads
        let mainnet_event_payload = sonic_rs::to_string(&mainnet_payload).unwrap();
        let canonicity_event_payload = sonic_rs::to_string(&canonicity_payload).unwrap();

        let mainnet_event = Event {
            event_type: EventType::MainnetBlock,
            payload: mainnet_event_payload,
        };

        let canonicity_event = Event {
            event_type: EventType::BlockCanonicityUpdate,
            payload: canonicity_event_payload,
        };

        // Subscribe to the shared publisher
        let mut receiver = shared_publisher.subscribe();

        // Send the events to the actor
        actor.handle_event(mainnet_event.clone()).await;
        actor.handle_event(canonicity_event.clone()).await;

        // Capture the published event
        if let Ok(published_event) = receiver.recv().await {
            assert_eq!(published_event.event_type, EventType::BatchCanonicalUserCommandLog);

            // Verify the published payload
            let published_payload: BatchCanonicalUserCommandLogPayload = sonic_rs::from_str(&published_event.payload).expect("Failed to deserialize payload");
            assert_eq!(published_payload.height, canonicity_payload.height);
            assert_eq!(published_payload.state_hash, canonicity_payload.state_hash);
            assert!(published_payload.canonical);

            // Verify the user commands
            assert_eq!(published_payload.commands.len(), user_commands.len());
            for (expected, actual) in user_commands.iter().zip(published_payload.commands.iter()) {
                assert_eq!(expected.memo, actual.memo);
                assert_eq!(expected.fee_payer, actual.fee_payer);
                assert_eq!(expected.sender, actual.sender);
                assert_eq!(expected.receiver, actual.receiver);
                assert_eq!(expected.status, actual.status);
                assert_eq!(expected.txn_type, actual.txn_type);
                assert_eq!(expected.nonce, actual.nonce);
                assert_eq!(expected.fee_nanomina, actual.fee_nanomina);
                assert_eq!(expected.amount_nanomina, actual.amount_nanomina);
            }
        } else {
            panic!("Expected CanonicalUserCommandLog event to be published.");
        }

        // Verify the event count
        assert_eq!(actor.actor_outputs().load(Ordering::SeqCst), 1);
    }
}
