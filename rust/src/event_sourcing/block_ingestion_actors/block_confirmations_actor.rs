use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    blockchain_tree::{BlockchainTree, Hash, Height, Node},
    event_sourcing::payloads::{BlockConfirmationPayload, NewBlockPayload},
};
use async_trait::async_trait;
use futures::lock::Mutex;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct BlockConfirmationsActor {
    id: String,
    shared_publisher: Arc<SharedPublisher>,
    events_published: AtomicUsize,
    blockchain_tree: Arc<Mutex<BlockchainTree>>,
}
impl BlockConfirmationsActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "BlockConfirmationsActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
            blockchain_tree: Arc::new(Mutex::new(BlockchainTree::new(11))),
        }
    }
}

#[async_trait]
impl Actor for BlockConfirmationsActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.events_published
    }

    async fn report(&self) {
        let tree = self.blockchain_tree.lock().await;
        self.print_report("Blockchain BTreeMap", tree.size());
    }

    async fn handle_event(&self, event: Event) {
        if event.event_type == EventType::NewBlock {
            let block_payload: NewBlockPayload = sonic_rs::from_str(&event.payload).unwrap();
            let mut blockchain_tree = self.blockchain_tree.lock().await;
            let next_node = Node {
                height: Height(block_payload.height),
                state_hash: Hash(block_payload.state_hash.clone()),
                previous_state_hash: Hash(block_payload.previous_state_hash),
                last_vrf_output: block_payload.last_vrf_output,
                metadata_str: Some("0".to_string()),
            };

            if blockchain_tree.is_empty() {
                blockchain_tree.set_root(next_node.clone()).unwrap();
                return;
            } else if blockchain_tree.has_parent(&next_node) {
                blockchain_tree.add_node(next_node.clone()).unwrap();
                let mut iter_node = next_node.clone();
                while let Some(parent) = blockchain_tree.get_parent_mut(&iter_node) {
                    if let Some(confirmations_str) = parent.metadata_str.clone() {
                        // Parse, increment, and update the confirmations count
                        let new_confirmations = confirmations_str.parse::<u8>().unwrap_or(0) + 1;
                        let metadata_str = Some(new_confirmations.to_string());
                        parent.metadata_str = metadata_str;

                        // Publish the confirmation event
                        if new_confirmations == 10 {
                            self.publish(Event {
                                event_type: EventType::BlockConfirmation,
                                payload: sonic_rs::to_string(&BlockConfirmationPayload {
                                    height: parent.height.0,
                                    state_hash: parent.state_hash.0.to_string(),
                                    confirmations: new_confirmations,
                                })
                                .unwrap(),
                            });
                        }
                    }
                    iter_node = parent.clone();
                }

                blockchain_tree.prune_tree().unwrap();
            } else {
                println!(
                    "Attempted to add block and height {} and state_hash {} but found no parent",
                    next_node.height.0, next_node.state_hash.0
                )
            }
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[cfg(test)]
mod block_confirmations_actor_tests {
    use super::*;
    use crate::event_sourcing::{
        events::{Event, EventType},
        payloads::{BlockConfirmationPayload, NewBlockPayload},
    };
    use std::sync::Arc;
    use tokio::time::timeout;

    // Helper function to set up the actor and subscriber
    fn setup_actor() -> (Arc<BlockConfirmationsActor>, tokio::sync::broadcast::Receiver<Event>) {
        let shared_publisher = Arc::new(SharedPublisher::new(200));
        let actor = Arc::new(BlockConfirmationsActor::new(Arc::clone(&shared_publisher)));
        let receiver = shared_publisher.subscribe();
        (actor, receiver)
    }

    #[tokio::test]
    async fn test_add_root_node() {
        let (actor, mut receiver) = setup_actor();

        let payload = NewBlockPayload {
            height: 0,
            state_hash: "root_hash".to_string(),
            previous_state_hash: "".to_string(),
            last_vrf_output: "vrf_output".to_string(),
        };

        let event = Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&payload).unwrap(),
        };

        actor.handle_event(event).await;

        // No confirmations should be published for the root node
        let published_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(published_event.is_err(), "No confirmations should be published for the root node.");
    }

    #[tokio::test]
    async fn test_block_confirmations_actor_last_updates() {
        let shared_publisher = Arc::new(SharedPublisher::new(200));
        let actor = BlockConfirmationsActor::new(Arc::clone(&shared_publisher));
        let mut receiver = shared_publisher.subscribe();

        // Add the first 10 blocks to the blockchain actor
        for i in 0..11 {
            let block_event = Event {
                event_type: EventType::NewBlock,
                payload: sonic_rs::to_string(&NewBlockPayload {
                    height: i, // up to i==10
                    state_hash: format!("hash_{}", i),
                    previous_state_hash: if i == 0 { "".to_string() } else { format!("hash_{}", i - 1) },
                    last_vrf_output: format!("vrf_output_{}", i),
                })
                .unwrap(),
            };
            actor.handle_event(block_event).await;
        }

        // Flush the receiver by draining all published events so far
        while timeout(std::time::Duration::from_millis(50), receiver.recv()).await.is_ok() {}

        // Add the 11th block to the blockchain actor
        let block_event = Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&NewBlockPayload {
                height: 11,
                state_hash: "hash_11".to_string(),
                previous_state_hash: "hash_10".to_string(),
                last_vrf_output: "vrf_output_11".to_string(),
            })
            .unwrap(),
        };
        actor.handle_event(block_event).await;

        // Collect confirmation events for the 10th block
        let mut received_events = vec![];

        // listen to 10+1 events
        if let Ok(event) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            received_events.push(event.unwrap());
        }

        // Verify the last ten confirmation events explicitly
        assert_eq!(received_events.len(), 1, "Expected 1 BlockConfirmation events for the 11th block.");

        // Check each individual event
        let expected_events = [(1, "hash_1", 10)];

        for (i, (expected_height, expected_hash, expected_confirmations)) in expected_events.iter().enumerate() {
            let confirmation_payload: BlockConfirmationPayload = sonic_rs::from_str(&received_events[i].payload).unwrap();

            assert_eq!(
                confirmation_payload.height, *expected_height as u64,
                "Confirmation event mismatch for height {}.",
                expected_height
            );
            assert_eq!(
                confirmation_payload.state_hash, *expected_hash,
                "State hash mismatch for height {}.",
                expected_height
            );
            assert_eq!(
                confirmation_payload.confirmations, *expected_confirmations as u8,
                "Confirmation count mismatch for block at height {}.",
                expected_height
            );
        }

        // Ensure the tree size is within the configured limit
        let tree = actor.blockchain_tree.lock().await;
        assert!(tree.size() <= 11, "Blockchain tree should not exceed 11 nodes (10 confirmations).");
    }
}
