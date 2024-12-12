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

pub struct CanonicalZkAppCommandLogActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
    pub canonical_items_manager: Arc<Mutex<CanonicalItemsManager<CanonicalBatchZkappCommandLogPayload>>>,
}

impl CanonicalZkAppCommandLogActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "CanonicalZkAppCommandLogActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
            canonical_items_manager: Arc::new(Mutex::new(CanonicalItemsManager::new((TRANSITION_FRONTIER_DISTANCE / 5usize) as u64))),
        }
    }
}

#[async_trait]
impl Actor for CanonicalZkAppCommandLogActor {
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
                            event_type: EventType::CanonicalBatchZkappCommandLog,
                            payload: sonic_rs::to_string(&payload).unwrap(),
                        });
                    }

                    if let Err(e) = manager.prune().await {
                        error!("{}", e);
                    }
                }
            }
            EventType::ZkAppCommandLog => {
                let event_payload: BatchZkappCommandLogPayload = sonic_rs::from_str(&event.payload).unwrap();
                {
                    let manager = self.canonical_items_manager.lock().await;
                    manager
                        .add_items_count(event_payload.height, &event_payload.state_hash, 1) // Expect only 1 item count
                        .await;
                    manager
                        .add_item(CanonicalBatchZkappCommandLogPayload {
                            canonical: false,     // Default value, will be updated based on canonicity
                            was_canonical: false, // Default value, will be updated based on previous canonicity
                            height: event_payload.height,
                            state_hash: event_payload.state_hash.clone(),
                            timestamp: event_payload.timestamp,
                            global_slot: event_payload.global_slot,
                            commands: event_payload.commands.clone(), // Clone commands for inclusion
                        })
                        .await;
                }
                {
                    let manager = self.canonical_items_manager.lock().await;
                    for payload in manager.get_updates(event_payload.height).await.iter() {
                        self.publish(Event {
                            event_type: EventType::CanonicalBatchZkappCommandLog,
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
mod canonical_zkapp_command_log_actor_tests {
    use super::*;
    use crate::event_sourcing::{
        events::{Event, EventType},
        models::{CommandStatus, CommandType, ZkAppCommandSummary},
        payloads::{BatchZkappCommandLogPayload, CanonicalBatchZkappCommandLogPayload},
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn test_publishes_after_conditions_met() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = CanonicalZkAppCommandLogActor::new(Arc::clone(&shared_publisher));

        let mut receiver = shared_publisher.subscribe();

        // Add a zkapp command log
        let zkapp_command_log = BatchZkappCommandLogPayload {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            timestamp: 123456,
            global_slot: 20,
            commands: vec![ZkAppCommandSummary {
                memo: "test_memo".to_string(),
                fee_payer: "payer_1".to_string(),
                status: CommandStatus::Applied,
                txn_type: CommandType::Payment,
                nonce: 1,
                fee_nanomina: 1000,
                account_updates: 2,
            }],
        };
        actor
            .handle_event(Event {
                event_type: EventType::ZkAppCommandLog,
                payload: sonic_rs::to_string(&zkapp_command_log).unwrap(),
            })
            .await;

        // Add a block canonicity update
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

        let payload: CanonicalBatchZkappCommandLogPayload = sonic_rs::from_str(&event.payload).unwrap();
        assert_eq!(payload.height, 10);
        assert_eq!(payload.state_hash, "state_hash_10");
        assert!(payload.canonical);
    }

    #[tokio::test]
    async fn test_does_not_publish_without_all_conditions_met() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = CanonicalZkAppCommandLogActor::new(Arc::clone(&shared_publisher));

        let mut receiver = shared_publisher.subscribe();

        // Add a zkapp command log with a mismatching state hash
        let zkapp_command_log = BatchZkappCommandLogPayload {
            height: 10,
            state_hash: "state_hash_other".to_string(),
            timestamp: 123456,
            global_slot: 20,
            commands: vec![ZkAppCommandSummary {
                memo: "test_memo".to_string(),
                fee_payer: "payer_1".to_string(),
                status: CommandStatus::Applied,
                txn_type: CommandType::Payment,
                nonce: 1,
                fee_nanomina: 1000,
                account_updates: 2,
            }],
        };
        actor
            .handle_event(Event {
                event_type: EventType::ZkAppCommandLog,
                payload: sonic_rs::to_string(&zkapp_command_log).unwrap(),
            })
            .await;

        // Add a block canonicity update
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
        assert!(no_event.is_err(), "No event should be published since conditions are not met");
    }
}
