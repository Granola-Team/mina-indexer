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

pub struct CanonicalUserCommandLogActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
    pub canonical_items_manager: Arc<Mutex<CanonicalItemsManager<CanonicalUserCommandLogPayload>>>,
}

impl CanonicalUserCommandLogActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "CanonicalUserCommandLogActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
            canonical_items_manager: Arc::new(Mutex::new(CanonicalItemsManager::new(TRANSITION_FRONTIER_DISTANCE as u64))),
        }
    }
}

#[async_trait]
impl Actor for CanonicalUserCommandLogActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.events_published
    }

    async fn report(&self) {
        let manager = self.canonical_items_manager.lock().await;
        self.print_report("CanonicalItemsManager", manager.get_len().await);
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
                            event_type: EventType::CanonicalUserCommandLog,
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
                    .add_items_count(event_payload.height, &event_payload.state_hash, event_payload.user_command_count as u64)
                    .await;
            }
            EventType::UserCommandLog => {
                let event_payload: UserCommandLogPayload = sonic_rs::from_str(&event.payload).unwrap();
                {
                    let manager = self.canonical_items_manager.lock().await;
                    manager
                        .add_item(CanonicalUserCommandLogPayload {
                            height: event_payload.height,
                            txn_hash: event_payload.txn_hash.to_string(),
                            state_hash: event_payload.state_hash.to_string(),
                            timestamp: event_payload.timestamp,
                            txn_type: event_payload.txn_type.clone(),
                            status: event_payload.status.clone(),
                            sender: event_payload.sender.to_string(),
                            receiver: event_payload.receiver.to_string(),
                            nonce: event_payload.nonce,
                            fee_nanomina: event_payload.fee_nanomina,
                            fee_payer: event_payload.fee_payer.to_string(),
                            amount_nanomina: event_payload.amount_nanomina,
                            canonical: false,     // use a default value
                            was_canonical: false, // use a default value
                        })
                        .await;
                }
                {
                    let manager = self.canonical_items_manager.lock().await;
                    for payload in manager.get_updates(event_payload.height).await.iter() {
                        self.publish(Event {
                            event_type: EventType::CanonicalUserCommandLog,
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
mod canonical_user_command_log_actor_tests {
    use super::*;
    use crate::stream::{
        events::{Event, EventType},
        mainnet_block_models::{CommandStatus, CommandType},
        payloads::{CanonicalUserCommandLogPayload, MainnetBlockPayload, UserCommandLogPayload},
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn test_publishes_after_all_conditions_met() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = CanonicalUserCommandLogActor::new(Arc::clone(&shared_publisher));

        let mut receiver = shared_publisher.subscribe();

        // Add a mainnet block with user command count
        let mainnet_block = MainnetBlockPayload {
            height: 10,
            user_command_count: 2,
            state_hash: "state_hash_10".to_string(),
            ..Default::default()
        };
        actor
            .handle_event(Event {
                event_type: EventType::MainnetBlock,
                payload: sonic_rs::to_string(&mainnet_block).unwrap(),
            })
            .await;

        // Add two user command logs
        let user_command_1 = UserCommandLogPayload {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            txn_hash: "txn_hash_1".to_string(),
            timestamp: 123456,
            txn_type: CommandType::Payment,
            status: CommandStatus::Applied,
            sender: "sender_1".to_string(),
            receiver: "receiver_1".to_string(),
            nonce: 1,
            fee_nanomina: 1000,
            fee_payer: "payer_1".to_string(),
            amount_nanomina: 5000,
        };
        let user_command_2 = UserCommandLogPayload {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            txn_hash: "txn_hash_2".to_string(),
            timestamp: 123456,
            txn_type: CommandType::StakeDelegation,
            status: CommandStatus::Applied,
            sender: "sender_2".to_string(),
            receiver: "receiver_2".to_string(),
            nonce: 2,
            fee_nanomina: 2000,
            fee_payer: "payer_2".to_string(),
            amount_nanomina: 7000,
        };
        actor
            .handle_event(Event {
                event_type: EventType::UserCommandLog,
                payload: sonic_rs::to_string(&user_command_1).unwrap(),
            })
            .await;
        actor
            .handle_event(Event {
                event_type: EventType::UserCommandLog,
                payload: sonic_rs::to_string(&user_command_2).unwrap(),
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

        let payload: CanonicalUserCommandLogPayload = sonic_rs::from_str(&event.payload).unwrap();
        assert_eq!(payload.height, 10);
        assert_eq!(payload.state_hash, "state_hash_10");
        assert!(payload.canonical);
    }

    #[tokio::test]
    async fn test_does_not_publish_without_all_conditions_met() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = CanonicalUserCommandLogActor::new(Arc::clone(&shared_publisher));

        let mut receiver = shared_publisher.subscribe();

        // Add a mainnet block with user command count
        let mainnet_block = MainnetBlockPayload {
            height: 10,
            user_command_count: 2,
            state_hash: "state_hash_other".to_string(), //no matching state hash
            ..Default::default()
        };
        actor
            .handle_event(Event {
                event_type: EventType::MainnetBlock,
                payload: sonic_rs::to_string(&mainnet_block).unwrap(),
            })
            .await;

        // Add one user command log (not enough)
        let user_command = UserCommandLogPayload {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            txn_hash: "txn_hash_1".to_string(),
            timestamp: 123456,
            txn_type: CommandType::Payment,
            status: CommandStatus::Applied,
            sender: "sender_1".to_string(),
            receiver: "receiver_1".to_string(),
            nonce: 1,
            fee_nanomina: 1000,
            fee_payer: "payer_1".to_string(),
            amount_nanomina: 5000,
        };
        actor
            .handle_event(Event {
                event_type: EventType::UserCommandLog,
                payload: sonic_rs::to_string(&user_command).unwrap(),
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
}
