use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    constants::GENESIS_STATE_HASH,
    stream::{
        models::{PreviousStateHash, StateHash},
        payloads::{BlockAncestorPayload, NewBlockAddedPayload},
    },
};
use async_trait::async_trait;
use futures::lock::Mutex;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc},
};

pub struct BlockchainTreeActor {
    id: String,
    shared_publisher: Arc<SharedPublisher>,
    events_processed: AtomicUsize,
    blockchain_tree: Arc<Mutex<HashMap<StateHash, PreviousStateHash>>>,
}

/// Publishes blocks as they are connected to the blockchain tree
impl BlockchainTreeActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "BlockchainTreeActor".to_string(),
            shared_publisher,
            events_processed: AtomicUsize::new(0),
            blockchain_tree: Arc::new(Mutex::new(HashMap::from([(
                StateHash(GENESIS_STATE_HASH.to_string()),
                PreviousStateHash("".to_string()),
            )]))),
        }
    }
}

#[async_trait]
impl Actor for BlockchainTreeActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn events_processed(&self) -> &AtomicUsize {
        &self.events_processed
    }

    async fn handle_event(&self, event: Event) {
        if event.event_type == EventType::BlockAncestor {
            let block_payload: BlockAncestorPayload = sonic_rs::from_str(&event.payload).unwrap();

            let new_block = StateHash(block_payload.state_hash.clone());
            let previous_block = PreviousStateHash(block_payload.previous_state_hash.clone());

            let mut blockchain_tree = self.blockchain_tree.lock().await;
            if blockchain_tree.contains_key(&StateHash(block_payload.previous_state_hash)) {
                // Insert the new block into the main tree
                blockchain_tree.insert(new_block.clone(), previous_block.clone());
                drop(blockchain_tree); // Drop the lock before proceeding

                // Publish the addition of the new block
                let added_payload = NewBlockAddedPayload {
                    height: block_payload.height,
                    state_hash: new_block.0.clone(),
                    previous_state_hash: previous_block.0,
                };
                self.publish(Event {
                    event_type: EventType::BlockAddedToTree,
                    payload: sonic_rs::to_string(&added_payload).unwrap(),
                });
                self.incr_event_processed();
            } else {
                self.publish(Event {
                    event_type: EventType::BlockAncestor,
                    payload: event.payload,
                });
            }
        }
    }

    fn publish(&self, event: Event) {
        self.shared_publisher.publish(event);
    }
}

#[tokio::test]
async fn test_blockchain_tree_actor_connects_blocks_in_order() {
    use crate::stream::shared_publisher::SharedPublisher;
    use std::sync::Arc;

    let shared_publisher = Arc::new(SharedPublisher::new(100));
    let actor = BlockchainTreeActor::new(Arc::clone(&shared_publisher));
    let mut receiver = shared_publisher.subscribe();

    // Connect blocks in order after the GENESIS block
    let blocks = vec![
        BlockAncestorPayload {
            height: 2,
            state_hash: "3N8aBlock1".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
        },
        BlockAncestorPayload {
            height: 3,
            state_hash: "3N8aBlock2".to_string(),
            previous_state_hash: "3N8aBlock1".to_string(),
        },
    ];

    for block in &blocks {
        let event = Event {
            event_type: EventType::BlockAncestor,
            payload: sonic_rs::to_string(block).unwrap(),
        };
        actor.handle_event(event).await;
    }

    // Check that each block is published in ascending order
    let mut last_height = 1; // GENESIS is at height 1
    for _ in 0..blocks.len() {
        if let Ok(event) = receiver.recv().await {
            let payload: NewBlockAddedPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(payload.height, last_height + 1);
            last_height = payload.height;
        }
    }
    assert_eq!(last_height, 3, "All blocks should have been published in order");
}

#[tokio::test]
async fn test_blockchain_tree_actor_rebroadcasts_unconnected_blocks() {
    use crate::stream::shared_publisher::SharedPublisher;
    use std::sync::Arc;

    let shared_publisher = Arc::new(SharedPublisher::new(100));
    let actor = BlockchainTreeActor::new(Arc::clone(&shared_publisher));
    let mut receiver = shared_publisher.subscribe();

    // Send an unconnected block (no known parent in the blockchain tree)
    let unconnected_block = BlockAncestorPayload {
        height: 2,
        state_hash: "3N8aBlock1".to_string(),
        previous_state_hash: "NonExistentParent".to_string(),
    };

    let event = Event {
        event_type: EventType::BlockAncestor,
        payload: sonic_rs::to_string(&unconnected_block).unwrap(),
    };

    actor.handle_event(event).await;

    // Check that the unconnected block was rebroadcasted
    if let Ok(event) = receiver.recv().await {
        assert_eq!(event.event_type, EventType::BlockAncestor);
        let payload: BlockAncestorPayload = sonic_rs::from_str(&event.payload).unwrap();
        assert_eq!(payload, unconnected_block);
    }
}

#[tokio::test]
async fn test_blockchain_tree_actor_reconnects_when_ancestor_arrives() {
    use crate::stream::shared_publisher::SharedPublisher;
    use std::sync::Arc;

    let shared_publisher = Arc::new(SharedPublisher::new(100));
    let actor = BlockchainTreeActor::new(Arc::clone(&shared_publisher));
    let mut receiver = shared_publisher.subscribe();

    // Send a disconnected block that references a non-existing ancestor
    let unconnected_block = BlockAncestorPayload {
        height: 3,
        state_hash: "3N8aBlock2".to_string(),
        previous_state_hash: "3N8aBlock1".to_string(),
    };

    let event = Event {
        event_type: EventType::BlockAncestor,
        payload: sonic_rs::to_string(&unconnected_block).unwrap(),
    };

    actor.handle_event(event).await;

    // Check the block is rebroadcasted
    if let Ok(event) = receiver.recv().await {
        assert_eq!(event.event_type, EventType::BlockAncestor);
        let payload: BlockAncestorPayload = sonic_rs::from_str(&event.payload).unwrap();
        assert_eq!(payload, unconnected_block);
    }

    // Now send the connecting ancestor block (height 2)
    let connecting_block = BlockAncestorPayload {
        height: 2,
        state_hash: "3N8aBlock1".to_string(),
        previous_state_hash: GENESIS_STATE_HASH.to_string(),
    };

    let event = Event {
        event_type: EventType::BlockAncestor,
        payload: sonic_rs::to_string(&connecting_block).unwrap(),
    };

    actor.handle_event(event).await;

    // Simulate rebroadcast of the unconnected block (height 3)
    let event = Event {
        event_type: EventType::BlockAncestor,
        payload: sonic_rs::to_string(&unconnected_block).unwrap(),
    };

    actor.handle_event(event).await;

    // Verify that both blocks are now processed in order
    let mut last_height = 1; // GENESIS is at height 1
    for _ in 0..2 {
        if let Ok(event) = receiver.recv().await {
            if event.event_type == EventType::BlockAddedToTree {
                let payload: NewBlockAddedPayload = sonic_rs::from_str(&event.payload).unwrap();
                assert_eq!(payload.height, last_height + 1);
                last_height = payload.height;
            }
        }
    }
    assert_eq!(last_height, 3, "All blocks should be published in the correct order after reconnection");
}
