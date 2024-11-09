use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    blockchain_tree::{BlockchainTree, Hash, Height, Node},
    constants::TRANSITION_FRONTIER_DISTANCE,
    stream::payloads::{BlockAncestorPayload, GenesisBlockPayload, NewBlockAddedPayload},
};
use async_trait::async_trait;
use futures::lock::Mutex;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct BlockchainTreeActor {
    id: String,
    shared_publisher: Arc<SharedPublisher>,
    events_processed: AtomicUsize,
    blockchain_tree: Arc<Mutex<BlockchainTree>>,
}

/// Publishes blocks as they are connected to the blockchain tree
impl BlockchainTreeActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "BlockchainTreeActor".to_string(),
            shared_publisher,
            events_processed: AtomicUsize::new(0),
            blockchain_tree: Arc::new(Mutex::new(BlockchainTree::new(TRANSITION_FRONTIER_DISTANCE))),
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
        match event.event_type {
            EventType::BlockAncestor => {
                let block_payload: BlockAncestorPayload = sonic_rs::from_str(&event.payload).unwrap();
                let mut blockchain_tree = self.blockchain_tree.lock().await;
                let next_node = Node {
                    height: Height(block_payload.height),
                    state_hash: Hash(block_payload.state_hash.clone()),
                    previous_state_hash: Hash(block_payload.previous_state_hash.clone()),
                    last_vrf_output: block_payload.last_vrf_output.clone(),
                };
                if blockchain_tree.has_parent(&next_node) {
                    blockchain_tree.add_node(next_node).unwrap();
                    let added_payload = NewBlockAddedPayload {
                        height: block_payload.height,
                        state_hash: block_payload.state_hash,
                        previous_state_hash: block_payload.previous_state_hash,
                        last_vrf_output: block_payload.last_vrf_output,
                    };
                    self.publish(Event {
                        event_type: EventType::BlockAddedToTree,
                        payload: sonic_rs::to_string(&added_payload).unwrap(),
                    });
                    self.incr_event_processed();
                } else {
                    // try again later
                    self.publish(Event {
                        event_type: EventType::BlockAncestor,
                        payload: event.payload,
                    });
                }
                blockchain_tree.prune_tree().unwrap();
            }
            EventType::GenesisBlock => {
                let genesis_payload: GenesisBlockPayload = sonic_rs::from_str(&event.payload).unwrap();
                let mut blockchain_tree = self.blockchain_tree.lock().await;
                let root_node = Node {
                    height: Height(genesis_payload.height),
                    state_hash: Hash(genesis_payload.state_hash.clone()),
                    previous_state_hash: Hash(genesis_payload.previous_state_hash.clone()),
                    last_vrf_output: genesis_payload.last_vrf_output.clone(),
                };
                blockchain_tree.set_root(root_node).unwrap();
                let added_payload = NewBlockAddedPayload {
                    height: genesis_payload.height,
                    state_hash: genesis_payload.state_hash,
                    previous_state_hash: genesis_payload.previous_state_hash,
                    last_vrf_output: genesis_payload.last_vrf_output,
                };
                self.publish(Event {
                    event_type: EventType::BlockAddedToTree,
                    payload: sonic_rs::to_string(&added_payload).unwrap(),
                });
                self.incr_event_processed();
            }
            _ => {}
        }
    }

    fn publish(&self, event: Event) {
        self.shared_publisher.publish(event);
    }
}

#[tokio::test]
async fn test_blockchain_tree_actor_connects_blocks_in_order() {
    use super::super::events::EventType;
    use crate::{constants::GENESIS_STATE_HASH, stream::shared_publisher::SharedPublisher};
    use std::sync::Arc;

    let shared_publisher = Arc::new(SharedPublisher::new(100));
    let actor = BlockchainTreeActor::new(Arc::clone(&shared_publisher));

    actor
        .handle_event(Event {
            event_type: EventType::GenesisBlock,
            payload: sonic_rs::to_string(&GenesisBlockPayload::new()).unwrap(),
        })
        .await;
    let mut receiver = shared_publisher.subscribe();

    // Connect blocks in order after the GENESIS block
    let blocks = vec![
        BlockAncestorPayload {
            height: 2,
            state_hash: "3N8aBlock1".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "".to_string(),
        },
        BlockAncestorPayload {
            height: 3,
            state_hash: "3N8aBlock2".to_string(),
            previous_state_hash: "3N8aBlock1".to_string(),
            last_vrf_output: "".to_string(),
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
    use super::super::events::EventType;
    use crate::stream::shared_publisher::SharedPublisher;
    use std::sync::Arc;

    let shared_publisher = Arc::new(SharedPublisher::new(100));
    let actor = BlockchainTreeActor::new(Arc::clone(&shared_publisher));

    actor
        .handle_event(Event {
            event_type: EventType::GenesisBlock,
            payload: sonic_rs::to_string(&GenesisBlockPayload::new()).unwrap(),
        })
        .await;

    let mut receiver = shared_publisher.subscribe();

    // Send an unconnected block (no known parent in the blockchain tree)
    let unconnected_block = BlockAncestorPayload {
        height: 2,
        state_hash: "3N8aBlock1".to_string(),
        previous_state_hash: "NonExistentParent".to_string(),
        last_vrf_output: "".to_string(),
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
    use crate::{constants::GENESIS_STATE_HASH, stream::shared_publisher::SharedPublisher};
    use std::sync::Arc;

    let shared_publisher = Arc::new(SharedPublisher::new(100));
    let actor = BlockchainTreeActor::new(Arc::clone(&shared_publisher));

    actor
        .handle_event(Event {
            event_type: EventType::GenesisBlock,
            payload: sonic_rs::to_string(&GenesisBlockPayload::new()).unwrap(),
        })
        .await;
    let mut receiver = shared_publisher.subscribe();

    // Send a disconnected block that references a non-existing ancestor
    let unconnected_block = BlockAncestorPayload {
        height: 3,
        state_hash: "3N8aBlock2".to_string(),
        previous_state_hash: "3N8aBlock1".to_string(),
        last_vrf_output: "".to_string(),
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
        last_vrf_output: "".to_string(),
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

#[tokio::test]
async fn test_blockchain_tree_actor_adds_genesis_block() {
    use crate::stream::shared_publisher::SharedPublisher;
    use std::sync::Arc;

    // Initialize shared publisher and actor
    let shared_publisher = Arc::new(SharedPublisher::new(100));
    let actor = BlockchainTreeActor::new(Arc::clone(&shared_publisher));
    let mut receiver = shared_publisher.subscribe();

    // Create and send the genesis block event
    let genesis_payload = GenesisBlockPayload::new();
    let genesis_event = Event {
        event_type: EventType::GenesisBlock,
        payload: sonic_rs::to_string(&genesis_payload).unwrap(),
    };

    actor.handle_event(genesis_event).await;

    // Verify that the genesis block was published as the first block
    if let Ok(event) = receiver.recv().await {
        assert_eq!(event.event_type, EventType::BlockAddedToTree);
        let payload: NewBlockAddedPayload = sonic_rs::from_str(&event.payload).unwrap();
        assert_eq!(payload.height, genesis_payload.height);
        assert_eq!(payload.state_hash, genesis_payload.state_hash);
        assert_eq!(payload.previous_state_hash, genesis_payload.previous_state_hash);
    } else {
        panic!("Expected the genesis block to be published, but no event was received");
    }

    // Verify that the genesis block has been processed
    assert_eq!(
        actor.events_processed().load(std::sync::atomic::Ordering::SeqCst),
        1,
        "Expected the genesis block to be processed once"
    );
}
