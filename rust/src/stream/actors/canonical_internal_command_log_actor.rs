use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    constants::TRANSITION_FRONTIER_DISTANCE,
    stream::{canonical_items_manager::CanonicalItemsManager, payloads::*},
};
use async_trait::async_trait;
use futures::lock::Mutex;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct CanonicalInternalCommandLogActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
    pub canonical_items_manager: Arc<Mutex<CanonicalItemsManager<CanonicalInternalCommandLogPayload>>>,
}

impl CanonicalInternalCommandLogActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "CanonicalInternalCommandLogActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
            canonical_items_manager: Arc::new(Mutex::new(CanonicalItemsManager::new((TRANSITION_FRONTIER_DISTANCE / 5usize) as u64))),
        }
    }
}

#[async_trait]
impl Actor for CanonicalInternalCommandLogActor {
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
                            event_type: EventType::CanonicalInternalCommandLog,
                            payload: sonic_rs::to_string(&payload).unwrap(),
                        });
                    }
                    manager.prune().await;
                }
            }
            EventType::MainnetBlock => {
                let event_payload: MainnetBlockPayload = sonic_rs::from_str(&event.payload).unwrap();
                let manager = self.canonical_items_manager.lock().await;
                manager
                    .add_items_count(event_payload.height, &event_payload.state_hash, event_payload.internal_command_count as u64)
                    .await;
            }
            EventType::InternalCommandLog => {
                let event_payload: InternalCommandLogPayload = sonic_rs::from_str(&event.payload).unwrap();
                {
                    let manager = self.canonical_items_manager.lock().await;
                    manager
                        .add_item(CanonicalInternalCommandLogPayload {
                            internal_command_type: event_payload.internal_command_type.clone(),
                            height: event_payload.height,
                            state_hash: event_payload.state_hash.to_string(),
                            timestamp: event_payload.timestamp,
                            amount_nanomina: event_payload.amount_nanomina,
                            recipient: event_payload.recipient.to_string(),
                            source: event_payload.source.clone(),
                            canonical: false,     // use a default value
                            was_canonical: false, // use a default value
                        })
                        .await;
                }
                {
                    let manager = self.canonical_items_manager.lock().await;
                    for payload in manager.get_updates(event_payload.height).await.iter() {
                        self.publish(Event {
                            event_type: EventType::CanonicalInternalCommandLog,
                            payload: sonic_rs::to_string(&payload).unwrap(),
                        });
                    }
                    manager.prune().await;
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
mod canonical_internal_command_log_actor_tests {
    use super::*;
    use crate::stream::{
        events::{Event, EventType},
        payloads::{CanonicalInternalCommandLogPayload, InternalCommandLogPayload, MainnetBlockPayload},
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn test_publishes_after_all_conditions_met() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = CanonicalInternalCommandLogActor::new(Arc::clone(&shared_publisher));

        let mut receiver = shared_publisher.subscribe();

        // Add a mainnet block with internal command count
        let mainnet_block = MainnetBlockPayload {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            internal_command_count: 2,
            ..Default::default()
        };
        actor
            .handle_event(Event {
                event_type: EventType::MainnetBlock,
                payload: sonic_rs::to_string(&mainnet_block).unwrap(),
            })
            .await;

        // Add two internal command logs
        let internal_command_1 = InternalCommandLogPayload {
            internal_command_type: InternalCommandType::Coinbase,
            height: 10,
            state_hash: "state_hash_10".to_string(),
            timestamp: 123456,
            amount_nanomina: 1_000_000,
            recipient: "recipient_1".to_string(),
            source: Some("source_1".to_string()),
        };
        let internal_command_2 = InternalCommandLogPayload {
            internal_command_type: InternalCommandType::FeeTransfer,
            height: 10,
            state_hash: "state_hash_10".to_string(),
            timestamp: 123456,
            amount_nanomina: 2_000_000,
            recipient: "recipient_2".to_string(),
            source: None,
        };
        actor
            .handle_event(Event {
                event_type: EventType::InternalCommandLog,
                payload: sonic_rs::to_string(&internal_command_1).unwrap(),
            })
            .await;
        actor
            .handle_event(Event {
                event_type: EventType::InternalCommandLog,
                payload: sonic_rs::to_string(&internal_command_2).unwrap(),
            })
            .await;

        // Add a block canonicity update for the same height
        let update = BlockCanonicityUpdatePayload {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            canonical: true,
            was_canonical: false,
        };
        actor
            .handle_event(Event {
                event_type: EventType::BlockCanonicityUpdate,
                payload: sonic_rs::to_string(&update).unwrap(),
            })
            .await;

        // Expect the event to be published
        let event = tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv())
            .await
            .expect("Expected a published event")
            .expect("Event received");

        let payload: CanonicalInternalCommandLogPayload = sonic_rs::from_str(&event.payload).unwrap();
        assert_eq!(payload.height, 10);
        assert_eq!(payload.state_hash, "state_hash_10");
        assert_eq!(payload.recipient, "recipient_1");
        assert!(payload.canonical);
    }

    #[tokio::test]
    async fn test_does_not_publish_without_all_conditions_met() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = CanonicalInternalCommandLogActor::new(Arc::clone(&shared_publisher));

        let mut receiver = shared_publisher.subscribe();

        // Add a mainnet block with internal command count
        let mainnet_block = MainnetBlockPayload {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            internal_command_count: 2, // one more than we intend to publish
            ..Default::default()
        };
        actor
            .handle_event(Event {
                event_type: EventType::MainnetBlock,
                payload: sonic_rs::to_string(&mainnet_block).unwrap(),
            })
            .await;

        // Add one internal command log (not enough)
        let internal_command = InternalCommandLogPayload {
            internal_command_type: InternalCommandType::Coinbase,
            height: 10,
            state_hash: "state_hash_10".to_string(),
            timestamp: 123456,
            amount_nanomina: 1_000_000,
            recipient: "recipient_1".to_string(),
            source: Some("source_1".to_string()),
        };
        actor
            .handle_event(Event {
                event_type: EventType::InternalCommandLog,
                payload: sonic_rs::to_string(&internal_command).unwrap(),
            })
            .await;

        // Add a block canonicity update for the same height
        let update = BlockCanonicityUpdatePayload {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            canonical: true,
            was_canonical: false,
        };
        actor
            .handle_event(Event {
                event_type: EventType::BlockCanonicityUpdate,
                payload: sonic_rs::to_string(&update).unwrap(),
            })
            .await;

        // Expect no event to be published
        let no_event = tokio::time::timeout(std::time::Duration::from_millis(100), receiver.recv()).await;
        assert!(no_event.is_err(), "No event should be published since not all conditions are met");
    }

    #[tokio::test]
    async fn test_multiple_state_hashes_handled_correctly() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = CanonicalInternalCommandLogActor::new(Arc::clone(&shared_publisher));

        let mut receiver = shared_publisher.subscribe();

        // Add a mainnet block
        let mainnet_block = MainnetBlockPayload {
            height: 10,
            state_hash: "state_hash_10_other".to_string(),
            internal_command_count: 1,
            ..Default::default()
        };
        actor
            .handle_event(Event {
                event_type: EventType::MainnetBlock,
                payload: sonic_rs::to_string(&mainnet_block).unwrap(),
            })
            .await;

        let mainnet_block = MainnetBlockPayload {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            internal_command_count: 1,
            ..Default::default()
        };
        actor
            .handle_event(Event {
                event_type: EventType::MainnetBlock,
                payload: sonic_rs::to_string(&mainnet_block).unwrap(),
            })
            .await;

        // Add internal command logs with different state hashes
        let command_1 = InternalCommandLogPayload {
            internal_command_type: InternalCommandType::Coinbase,
            height: 10,
            state_hash: "state_hash_10".to_string(),
            timestamp: 123456,
            amount_nanomina: 1_000_000,
            recipient: "recipient_1".to_string(),
            source: Some("source_1".to_string()),
        };
        let command_2 = InternalCommandLogPayload {
            internal_command_type: InternalCommandType::FeeTransfer,
            height: 10,
            state_hash: "state_hash_10_other".to_string(),
            timestamp: 123456,
            amount_nanomina: 2_000_000,
            recipient: "recipient_2".to_string(),
            source: None,
        };

        actor
            .handle_event(Event {
                event_type: EventType::InternalCommandLog,
                payload: sonic_rs::to_string(&command_1).unwrap(),
            })
            .await;
        actor
            .handle_event(Event {
                event_type: EventType::InternalCommandLog,
                payload: sonic_rs::to_string(&command_2).unwrap(),
            })
            .await;

        // Add block canonicity updates
        let update_1 = BlockCanonicityUpdatePayload {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            canonical: true,
            was_canonical: false,
        };
        let update_2 = BlockCanonicityUpdatePayload {
            height: 10,
            state_hash: "state_hash_10_other".to_string(),
            canonical: true,
            was_canonical: false,
        };
        actor
            .handle_event(Event {
                event_type: EventType::BlockCanonicityUpdate,
                payload: sonic_rs::to_string(&update_1).unwrap(),
            })
            .await;
        actor
            .handle_event(Event {
                event_type: EventType::BlockCanonicityUpdate,
                payload: sonic_rs::to_string(&update_2).unwrap(),
            })
            .await;

        // Verify both events are published
        let first_event = tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv())
            .await
            .expect("Expected a published event")
            .expect("Event received");

        let second_event = tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv())
            .await
            .expect("Expected a published event")
            .expect("Event received");

        let payload_1: CanonicalInternalCommandLogPayload = sonic_rs::from_str(&first_event.payload).unwrap();
        let payload_2: CanonicalInternalCommandLogPayload = sonic_rs::from_str(&second_event.payload).unwrap();

        assert!(payload_1.state_hash == "state_hash_10" || payload_2.state_hash == "state_hash_10");
        assert!(payload_1.state_hash == "state_hash_10_other" || payload_2.state_hash == "state_hash_10_other");
    }
}
