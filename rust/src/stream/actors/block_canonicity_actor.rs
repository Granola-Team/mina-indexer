use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    blockchain_tree::{BlockchainTree, Hash, Height, Node},
    constants::TRANSITION_FRONTIER_DISTANCE,
    stream::payloads::{BlockCanonicityUpdatePayload, NewBlockPayload},
};
use async_trait::async_trait;
use futures::lock::Mutex;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct BlockCanonicityActor {
    id: String,
    shared_publisher: Arc<SharedPublisher>,
    events_published: AtomicUsize,
    blockchain_tree: Arc<Mutex<BlockchainTree>>,
}
impl BlockCanonicityActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "BlockCanonicityActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
            blockchain_tree: Arc::new(Mutex::new(BlockchainTree::new(TRANSITION_FRONTIER_DISTANCE))),
        }
    }

    fn process_equal_height(&self, current_best_block: &Node, next_node: Node, next_best_block: &Node) {
        let is_canonical = current_best_block != next_best_block;
        if is_canonical {
            self.publish_canonical_update(current_best_block.clone(), false, true);
            self.publish_canonical_update(next_node, true, false);
        } else {
            self.publish_canonical_update(next_node, false, false);
        }
    }

    fn process_greater_height(&self, blockchain_tree: &mut BlockchainTree, current_best_block: &Node, next_node: Node) {
        let parent = blockchain_tree.get_parent(&next_node).unwrap();
        if parent != current_best_block {
            self.update_ancestries(blockchain_tree, current_best_block, &next_node);
        }
        self.publish_canonical_update(next_node, true, false);
    }

    fn update_ancestries(&self, blockchain_tree: &mut BlockchainTree, current_best_block: &Node, next_node: &Node) {
        let (prior_ancestry, mut new_ancestry, _) = blockchain_tree
            .get_shared_ancestry(current_best_block, blockchain_tree.get_parent(next_node).unwrap())
            .unwrap();

        for prior in prior_ancestry.iter() {
            self.publish_canonical_update(prior.clone(), false, true);
        }

        new_ancestry.reverse();
        for new_a in new_ancestry.iter() {
            self.publish_canonical_update(new_a.clone(), true, false);
        }
    }

    fn publish_canonical_update(&self, node: Node, canonical: bool, was_canonical: bool) {
        let update = BlockCanonicityUpdatePayload {
            height: node.height.0,
            state_hash: node.state_hash.0.clone(),
            canonical,
            was_canonical,
        };
        self.publish_event(update);
    }

    fn publish_event(&self, update: BlockCanonicityUpdatePayload) {
        self.publish(Event {
            event_type: EventType::BlockCanonicityUpdate,
            payload: sonic_rs::to_string(&update).unwrap(),
        });
        self.incr_event_published();
    }
}

#[async_trait]
impl Actor for BlockCanonicityActor {
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
                ..Default::default()
            };
            if next_node.height.0 == 1 {
                blockchain_tree.set_root(next_node.clone()).unwrap();
                self.publish_canonical_update(next_node, true, false);
                return;
            }
            if blockchain_tree.has_parent(&next_node) {
                let (height, current_best_block) = blockchain_tree.get_best_tip().unwrap();
                blockchain_tree.add_node(next_node.clone()).unwrap();
                let (_, next_best_block) = blockchain_tree.get_best_tip().unwrap();

                match next_node.height.cmp(&height) {
                    std::cmp::Ordering::Equal => {
                        self.process_equal_height(&current_best_block, next_node, &next_best_block);
                    }
                    std::cmp::Ordering::Greater => {
                        self.process_greater_height(&mut blockchain_tree, &current_best_block, next_node);
                    }
                    std::cmp::Ordering::Less => {
                        self.publish_canonical_update(next_node, false, false);
                    }
                }
                blockchain_tree.prune_tree().unwrap();
            } else {
                panic!("Block received out of order");
            }
        }
    }

    fn publish(&self, event: Event) {
        self.shared_publisher.publish(event);
    }
}

#[tokio::test]
async fn test_non_canonical_block_with_vrf_info() -> anyhow::Result<()> {
    use crate::{
        constants::GENESIS_STATE_HASH,
        stream::payloads::{BlockCanonicityUpdatePayload, GenesisBlockPayload, NewBlockPayload},
    };
    use std::sync::atomic::Ordering;

    // Create shared publisher
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = BlockCanonicityActor::new(Arc::clone(&shared_publisher));

    // Create canonical and non-canonical block payloads at the same height
    let canonical_block_payload = NewBlockPayload {
        height: 2,
        state_hash: "canonical_hash".to_string(),
        previous_state_hash: GENESIS_STATE_HASH.to_string(),
        last_vrf_output: "b_vrf_output".to_string(),
    };
    let non_canonical_block_payload = NewBlockPayload {
        height: 2,
        state_hash: "non_canonical_hash".to_string(),
        previous_state_hash: GENESIS_STATE_HASH.to_string(),
        last_vrf_output: "a_vrf_output".to_string(),
    };

    actor
        .handle_event(Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&GenesisBlockPayload::new()).unwrap(),
        })
        .await;

    // Subscribe to the shared publisher to capture the output
    let mut receiver = shared_publisher.subscribe();

    // Handle the canonical block event first
    actor
        .handle_event(Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&canonical_block_payload).unwrap(),
        })
        .await;

    // Expect the first event for the canonical block
    if let Ok(received_event) = receiver.recv().await {
        assert_eq!(received_event.event_type, EventType::BlockCanonicityUpdate);

        // Deserialize the payload and check values
        let payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&received_event.payload).unwrap();
        assert_eq!(payload.height, 2);
        assert_eq!(payload.state_hash, "canonical_hash");
        assert!(payload.canonical); // Ensure the canonical block is marked as canonical
    } else {
        panic!("Expected a BlockCanonicityUpdate event for canonical block but did not receive one.");
    }

    // Handle the non-canonical block event, which should trigger a non-canonical update
    actor
        .handle_event(Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&non_canonical_block_payload).unwrap(),
        })
        .await;

    // Expect the second event for the non-canonical block
    if let Ok(received_event) = receiver.recv().await {
        assert_eq!(received_event.event_type, EventType::BlockCanonicityUpdate);

        // Deserialize the payload and check values
        let payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&received_event.payload).unwrap();
        assert_eq!(payload.height, 2);
        assert_eq!(payload.state_hash, "non_canonical_hash");
        assert!(!payload.canonical); // Ensure the non-canonical block is marked as non-canonical
    } else {
        panic!("Expected a BlockCanonicityUpdate event for non-canonical block but did not receive one.");
    }

    // Verify both events have been processed
    assert_eq!(actor.actor_outputs().load(Ordering::SeqCst), 3);

    Ok(())
}

#[tokio::test]
async fn test_new_block_becomes_canonical_over_existing_block() -> anyhow::Result<()> {
    use crate::{
        constants::GENESIS_STATE_HASH,
        stream::payloads::{BlockCanonicityUpdatePayload, GenesisBlockPayload, NewBlockPayload},
    };
    use std::sync::atomic::Ordering;

    // Create shared publisher and actor
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = BlockCanonicityActor::new(Arc::clone(&shared_publisher));

    // Set up canonical and non-canonical block payloads at the same height
    let initial_block_payload = NewBlockPayload {
        height: 2,
        state_hash: "initial_block_hash".to_string(),
        last_vrf_output: "a_vrf_output".to_string(),
        previous_state_hash: GENESIS_STATE_HASH.to_string(),
    };
    let new_canonical_block_payload = NewBlockPayload {
        height: 2,
        state_hash: "new_canonical_hash".to_string(),
        last_vrf_output: "b_vrf_output".to_string(),
        previous_state_hash: GENESIS_STATE_HASH.to_string(),
    };

    actor
        .handle_event(Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&GenesisBlockPayload::new()).unwrap(),
        })
        .await;

    // Subscribe to shared publisher to capture events
    let mut receiver = shared_publisher.subscribe();

    // Handle the initial block event, which should initially become canonical
    actor
        .handle_event(Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&initial_block_payload).unwrap(),
        })
        .await;

    // Expect the initial block to be published as canonical
    if let Ok(received_event) = receiver.recv().await {
        assert_eq!(received_event.event_type, EventType::BlockCanonicityUpdate);

        let payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&received_event.payload).unwrap();
        assert_eq!(payload.height, 2);
        assert_eq!(payload.state_hash, "initial_block_hash");
        assert!(payload.canonical);
    } else {
        panic!("Expected initial block to be canonical, but did not receive update.");
    }

    // Handle the new canonical block event with higher VRF, marking it as canonical
    actor
        .handle_event(Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&new_canonical_block_payload).unwrap(),
        })
        .await;

    // Expect the new block to take over as canonical, and the initial block to be non-canonical
    let mut received_new_canonical_update = false;
    let mut received_non_canonical_update = false;

    while let Ok(received_event) = receiver.recv().await {
        let payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&received_event.payload).unwrap();

        if payload.state_hash == "new_canonical_hash" && payload.canonical {
            received_new_canonical_update = true;
        } else if payload.state_hash == "initial_block_hash" && !payload.canonical {
            received_non_canonical_update = true;
        }

        if received_new_canonical_update && received_non_canonical_update {
            break;
        }
    }

    // Assert both canonical and non-canonical updates were received as expected
    assert!(received_new_canonical_update, "New block should be marked as canonical.");
    assert!(received_non_canonical_update, "Initial block should be marked as non-canonical.");
    assert_eq!(actor.actor_outputs().load(Ordering::SeqCst), 4);

    Ok(())
}

#[tokio::test]
async fn test_longer_branch_outcompetes_canonical_branch_with_tiebreaker() -> anyhow::Result<()> {
    use crate::{
        constants::GENESIS_STATE_HASH,
        stream::payloads::{BlockCanonicityUpdatePayload, GenesisBlockPayload, NewBlockPayload},
    };
    use std::sync::atomic::Ordering;

    // Create shared publisher
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = BlockCanonicityActor::new(Arc::clone(&shared_publisher));

    // Create canonical block payloads in the original branch
    let original_block1_payload = NewBlockPayload {
        height: 2,
        state_hash: "original_block_1".to_string(),
        previous_state_hash: GENESIS_STATE_HASH.to_string(),
        last_vrf_output: "b_vrf_output".to_string(),
    };
    let original_block2_payload = NewBlockPayload {
        height: 3,
        state_hash: "original_block_2".to_string(),
        previous_state_hash: "original_block_1".to_string(),
        last_vrf_output: "b_vrf_output".to_string(),
    };

    // Create competing branch payloads
    let competing_block1_payload = NewBlockPayload {
        height: 2,
        state_hash: "competing_block_1".to_string(),
        previous_state_hash: GENESIS_STATE_HASH.to_string(),
        last_vrf_output: "a_vrf_output".to_string(),
    };
    let competing_block2_payload = NewBlockPayload {
        height: 3,
        state_hash: "competing_block_2".to_string(),
        previous_state_hash: "competing_block_1".to_string(),
        last_vrf_output: "a_vrf_output".to_string(),
    };
    let competing_block3_payload = NewBlockPayload {
        height: 4,
        state_hash: "competing_block_3".to_string(),
        previous_state_hash: "competing_block_2".to_string(),
        last_vrf_output: "a_vrf_output".to_string(),
    };

    actor
        .handle_event(Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&GenesisBlockPayload::new()).unwrap(),
        })
        .await;

    // Subscribe to the shared publisher to capture the output
    let mut receiver = shared_publisher.subscribe();

    // Handle events in sequence: original branch then competing branch
    actor
        .handle_event(Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&original_block1_payload).unwrap(),
        })
        .await;
    actor
        .handle_event(Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&original_block2_payload).unwrap(),
        })
        .await;

    // Competing branch events to outcompete original branch
    actor
        .handle_event(Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&competing_block1_payload).unwrap(),
        })
        .await;
    actor
        .handle_event(Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&competing_block2_payload).unwrap(),
        })
        .await;
    actor
        .handle_event(Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&competing_block3_payload).unwrap(),
        })
        .await;

    // Expected sequence of events
    let expected_events = vec![
        // Initially, both original blocks are marked as canonical
        BlockCanonicityUpdatePayload {
            height: 2,
            state_hash: "original_block_1".to_string(),
            canonical: true,
            was_canonical: false,
        },
        BlockCanonicityUpdatePayload {
            height: 3,
            state_hash: "original_block_2".to_string(),
            canonical: true,
            was_canonical: false,
        },
        // Competing blocks are added as non-canonical until the tiebreaker
        BlockCanonicityUpdatePayload {
            height: 2,
            state_hash: "competing_block_1".to_string(),
            canonical: false,
            was_canonical: false,
        },
        BlockCanonicityUpdatePayload {
            height: 3,
            state_hash: "competing_block_2".to_string(),
            canonical: false,
            was_canonical: false,
        },
        // Competing branch wins, update the blocks
        BlockCanonicityUpdatePayload {
            height: 3,
            state_hash: "original_block_2".to_string(),
            canonical: false,
            was_canonical: true,
        },
        BlockCanonicityUpdatePayload {
            height: 2,
            state_hash: "original_block_1".to_string(),
            canonical: false,
            was_canonical: true,
        },
        BlockCanonicityUpdatePayload {
            height: 2,
            state_hash: "competing_block_1".to_string(),
            canonical: true,
            was_canonical: false,
        },
        BlockCanonicityUpdatePayload {
            height: 3,
            state_hash: "competing_block_2".to_string(),
            canonical: true,
            was_canonical: false,
        },
        BlockCanonicityUpdatePayload {
            height: 4,
            state_hash: "competing_block_3".to_string(),
            canonical: true,
            was_canonical: false,
        },
    ];

    // Verify the sequence of events
    let num_expected_events = expected_events.len();
    for expected_event in expected_events.into_iter() {
        let received_event = receiver.recv().await.expect("Expected event not received");
        assert_eq!(received_event.event_type, EventType::BlockCanonicityUpdate);

        let payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&received_event.payload).unwrap();
        assert_eq!(payload, expected_event);
    }

    let genesis_event = 1;
    assert_eq!(actor.actor_outputs().load(Ordering::SeqCst), num_expected_events + genesis_event);

    Ok(())
}
