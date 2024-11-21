use super::{super::shared_publisher::SharedPublisher, Actor};
use crate::{
    constants::TRANSITION_FRONTIER_DISTANCE,
    stream::{canonical_items_manager::CanonicalItemsManager, events::*, payloads::*},
};
use async_trait::async_trait;
use futures::lock::Mutex;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct CanonicalBlockLogActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
    pub canonical_items_manager: Arc<Mutex<CanonicalItemsManager<CanonicalBlockLogPayload>>>,
}

impl CanonicalBlockLogActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "CanonicalBlockLogActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
            canonical_items_manager: Arc::new(Mutex::new(CanonicalItemsManager::new((TRANSITION_FRONTIER_DISTANCE / 5usize) as u64))),
        }
    }
}

#[async_trait]
impl Actor for CanonicalBlockLogActor {
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
                            event_type: EventType::CanonicalBlockLog,
                            payload: sonic_rs::to_string(&payload).unwrap(),
                        });
                    }
                    if let Err(e) = manager.prune().await {
                        eprintln!("{}", e);
                    }
                }
            }
            EventType::BlockLog => {
                let event_payload: BlockLogPayload = sonic_rs::from_str(&event.payload).unwrap();
                {
                    let manager = self.canonical_items_manager.lock().await;
                    manager
                        .add_item(CanonicalBlockLogPayload {
                            height: event_payload.height,
                            state_hash: event_payload.state_hash.to_string(),
                            previous_state_hash: event_payload.previous_state_hash.to_string(),
                            user_command_count: event_payload.user_command_count,
                            snark_work_count: event_payload.snark_work_count,
                            timestamp: event_payload.timestamp,
                            coinbase_receiver: event_payload.coinbase_receiver.to_string(),
                            coinbase_reward_nanomina: event_payload.coinbase_reward_nanomina,
                            global_slot_since_genesis: event_payload.global_slot_since_genesis,
                            last_vrf_output: event_payload.last_vrf_output.to_string(),
                            is_berkeley_block: event_payload.is_berkeley_block,
                            canonical: false, // default value
                        })
                        .await;
                    manager.add_items_count(event_payload.height, &event_payload.state_hash, 1).await;
                }
                {
                    let manager = self.canonical_items_manager.lock().await;
                    for payload in manager.get_updates(event_payload.height).await.iter() {
                        self.publish(Event {
                            event_type: EventType::CanonicalBlockLog,
                            payload: sonic_rs::to_string(&payload).unwrap(),
                        });
                    }

                    if let Err(e) = manager.prune().await {
                        eprintln!("{}", e);
                    }
                }
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
mod canonical_block_log_actor_tests {
    use super::*;
    use crate::stream::events::{Event, EventType};
    use std::sync::Arc;
    use tokio::time::timeout;

    async fn setup_actor() -> (Arc<CanonicalBlockLogActor>, tokio::sync::broadcast::Receiver<Event>) {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = Arc::new(CanonicalBlockLogActor::new(Arc::clone(&shared_publisher)));
        let receiver = shared_publisher.subscribe();
        (actor, receiver)
    }

    #[tokio::test]
    async fn test_canonical_block_log() {
        let (actor, mut receiver) = setup_actor().await;

        // Add a BlockLog event
        let block_log_payload = BlockLogPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            previous_state_hash: "hash_0".to_string(),
            user_command_count: 10,
            snark_work_count: 5,
            timestamp: 1234567890,
            coinbase_receiver: "receiver".to_string(),
            coinbase_reward_nanomina: 1000,
            global_slot_since_genesis: 123,
            last_vrf_output: "vrf_output".to_string(),
            is_berkeley_block: true,
        };

        let block_log_event = Event {
            event_type: EventType::BlockLog,
            payload: sonic_rs::to_string(&block_log_payload).unwrap(),
        };

        actor.handle_event(block_log_event).await;

        // Send a BlockCanonicityUpdate event
        let block_canonicity_update_payload = BlockCanonicityUpdatePayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            canonical: true,
            was_canonical: false,
        };

        let block_canonicity_update_event = Event {
            event_type: EventType::BlockCanonicityUpdate,
            payload: sonic_rs::to_string(&block_canonicity_update_payload).unwrap(),
        };

        actor.handle_event(block_canonicity_update_event).await;

        // Confirm CanonicalBlockLog event was published
        let received_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(received_event.is_ok(), "Expected a CanonicalBlockLog event");

        let event = received_event.unwrap().unwrap();
        assert_eq!(event.event_type, EventType::CanonicalBlockLog);

        let canonical_payload: CanonicalBlockLogPayload = sonic_rs::from_str(&event.payload).unwrap();
        assert_eq!(canonical_payload.height, 1);
        assert_eq!(canonical_payload.state_hash, "hash_1");
        assert!(canonical_payload.canonical);
    }

    #[tokio::test]
    async fn test_canonical_block_log_different_event_order() {
        let (actor, mut receiver) = setup_actor().await;

        // Add a BlockLog event
        let block_log_payload = BlockLogPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            previous_state_hash: "hash_0".to_string(),
            user_command_count: 10,
            snark_work_count: 5,
            timestamp: 1234567890,
            coinbase_receiver: "receiver".to_string(),
            coinbase_reward_nanomina: 1000,
            global_slot_since_genesis: 123,
            last_vrf_output: "vrf_output".to_string(),
            is_berkeley_block: true,
        };

        let block_log_event = Event {
            event_type: EventType::BlockLog,
            payload: sonic_rs::to_string(&block_log_payload).unwrap(),
        };

        // Send a BlockCanonicityUpdate event
        let block_canonicity_update_payload = BlockCanonicityUpdatePayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            canonical: true,
            was_canonical: false,
        };

        let block_canonicity_update_event = Event {
            event_type: EventType::BlockCanonicityUpdate,
            payload: sonic_rs::to_string(&block_canonicity_update_payload).unwrap(),
        };

        actor.handle_event(block_canonicity_update_event).await;

        actor.handle_event(block_log_event).await;

        // Confirm CanonicalBlockLog event was published
        let received_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(received_event.is_ok(), "Expected a CanonicalBlockLog event");

        let event = received_event.unwrap().unwrap();
        assert_eq!(event.event_type, EventType::CanonicalBlockLog);

        let canonical_payload: CanonicalBlockLogPayload = sonic_rs::from_str(&event.payload).unwrap();
        assert_eq!(canonical_payload.height, 1);
        assert_eq!(canonical_payload.state_hash, "hash_1");
        assert!(canonical_payload.canonical);
    }
}
