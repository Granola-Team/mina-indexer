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

    fn process_equal_height(&self, blockchain_tree: &mut BlockchainTree, current_best_block: &Node, next_node: Node) {
        if BlockchainTree::greater(&next_node, current_best_block) {
            self.update_ancestries(blockchain_tree, current_best_block, &next_node);
        } else {
            self.publish_canonical_update(next_node, false, false);
        }
    }

    fn process_greater_height(&self, blockchain_tree: &mut BlockchainTree, current_best_block: &Node, next_node: Node) {
        let parent = blockchain_tree.get_parent(&next_node).unwrap();
        if parent.state_hash != current_best_block.state_hash {
            self.update_ancestries(blockchain_tree, current_best_block, &parent);
        }
        self.publish_canonical_update(next_node, true, false);
    }

    fn update_ancestries(&self, blockchain_tree: &BlockchainTree, current_best_block: &Node, next_node: &Node) {
        let (prior_ancestry, mut new_ancestry, _) = blockchain_tree.get_shared_ancestry(current_best_block, next_node).unwrap();

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
            if blockchain_tree.is_empty() {
                blockchain_tree.set_root(next_node.clone()).unwrap();
                self.publish_canonical_update(next_node, true, false);
                return;
            } else if blockchain_tree.has_parent(&next_node) {
                let (height, current_best_block) = blockchain_tree.get_best_tip().unwrap();
                blockchain_tree.add_node(next_node.clone()).unwrap();

                match next_node.height.cmp(&height) {
                    std::cmp::Ordering::Equal => {
                        self.process_equal_height(&mut blockchain_tree, &current_best_block, next_node);
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
                println!(
                    "Attempted to add block and height {} and state_hash {} but found no parent",
                    next_node.height.0, next_node.state_hash.0
                )
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

#[tokio::test]
async fn test_block_with_different_parent_at_same_height() -> anyhow::Result<()> {
    use crate::{
        constants::GENESIS_STATE_HASH,
        stream::payloads::{BlockCanonicityUpdatePayload, NewBlockPayload},
    };

    // Create shared publisher
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = BlockCanonicityActor::new(Arc::clone(&shared_publisher));

    // Root block payload
    let root_block = NewBlockPayload {
        height: 1228,
        state_hash: "3NLGaCf4zNXouyKDkmrFuYBVt8BHnCuLC9PMvkytfFgiuuqv5xsH".to_string(),
        previous_state_hash: GENESIS_STATE_HASH.to_string(),
        last_vrf_output: "QNWhrsZ3Ld1gsJTUGBUBCs9iQD00hKUvZkR1GeFjEgA=".to_string(),
    };

    // First branch payloads
    let branch1_block1 = NewBlockPayload {
        height: 1229,
        state_hash: "3NLU6jZai3xytD6PkpdKNFTYjfYSsEWoksr9AmufqYbxJN2Mpuke".to_string(),
        previous_state_hash: root_block.state_hash.clone(),
        last_vrf_output: "PhNdMGQWACdnDnOg3D75icmHf5Mu_0F44ua-fzz4DwA=".to_string(),
    };
    let branch1_block2 = NewBlockPayload {
        height: 1230,
        state_hash: "3NKNqeqRjSY8Qy1x4qwzfCVSCGUq2czp4Jo9E7pUY6ZnHTBg4JiW".to_string(),
        previous_state_hash: branch1_block1.state_hash.clone(),
        last_vrf_output: "h8fO60ijmmBdiMfCBSPz47vsRW8BHg2hnYo4iwl3BAA=".to_string(),
    };
    let branch1_block3 = NewBlockPayload {
        height: 1231,
        state_hash: "3NKMBzGM1pySPanKkLdxjnUS9mZ88oCeanq3Nhm2QZ6eNB5ZkeGn".to_string(),
        previous_state_hash: branch1_block2.state_hash.clone(),
        last_vrf_output: "h8fO60ijmmBdiMfCBSPz47vsRW8BHg2hnYo4iwl3BAA=".to_string(),
    };

    // Second branch payloads
    let branch2_block1 = NewBlockPayload {
        height: 1229,
        state_hash: "3NL63pX2kKi4pezYnm6MPjsDK9VSvJ1XFiUaed9vxPEa5PuWPLeZ".to_string(),
        previous_state_hash: root_block.state_hash.clone(),
        last_vrf_output: "hPuL8ZcZI2pIZhbVLVVD0U4CO0FLlnIgW_rYULV_HAA=".to_string(),
    };
    let branch2_block2 = NewBlockPayload {
        height: 1230,
        state_hash: "3NKcSppgUnmuGp9kGQdp7ZXUh5vcf7rm9mhKE4UMvwcuUaB8wb7S".to_string(),
        previous_state_hash: branch2_block1.state_hash.clone(),
        last_vrf_output: "Wl4EiJDCuzNCCn2w_6vDywRZStf4iNM5h4qUxXvOFAA=".to_string(),
    };
    let branch2_block3 = NewBlockPayload {
        height: 1231,
        state_hash: "3NLN2uxCoRsiB2C4uZHDcY6WxJDz6EEWmoruCFva74E4cgTYgJCQ".to_string(),
        previous_state_hash: branch2_block2.state_hash.clone(),
        last_vrf_output: "qQNTDpNokYfZs1wlQNcSLWazVCu43O7mLAhxlgwjBAA=".to_string(),
    };

    // Initialize with the root block
    actor
        .handle_event(Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&root_block).unwrap(),
        })
        .await;

    // Subscribe to shared publisher to capture events
    let mut receiver = shared_publisher.subscribe();

    // Process branch 1
    actor
        .handle_event(Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&branch1_block1).unwrap(),
        })
        .await;
    actor
        .handle_event(Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&branch1_block2).unwrap(),
        })
        .await;
    actor
        .handle_event(Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&branch1_block3).unwrap(),
        })
        .await;

    // Process branch 2
    actor
        .handle_event(Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&branch2_block1).unwrap(),
        })
        .await;
    actor
        .handle_event(Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&branch2_block2).unwrap(),
        })
        .await;
    actor
        .handle_event(Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&branch2_block3).unwrap(),
        })
        .await;

    let mut expected_events = vec![
        BlockCanonicityUpdatePayload {
            height: 1231,
            state_hash: "3NLN2uxCoRsiB2C4uZHDcY6WxJDz6EEWmoruCFva74E4cgTYgJCQ".to_string(),
            canonical: true,
            was_canonical: false,
        },
        BlockCanonicityUpdatePayload {
            height: 1230,
            state_hash: "3NKcSppgUnmuGp9kGQdp7ZXUh5vcf7rm9mhKE4UMvwcuUaB8wb7S".to_string(),
            canonical: true,
            was_canonical: false,
        },
        BlockCanonicityUpdatePayload {
            height: 1229,
            state_hash: "3NL63pX2kKi4pezYnm6MPjsDK9VSvJ1XFiUaed9vxPEa5PuWPLeZ".to_string(),
            canonical: true,
            was_canonical: false,
        },
        BlockCanonicityUpdatePayload {
            height: 1229,
            state_hash: "3NLU6jZai3xytD6PkpdKNFTYjfYSsEWoksr9AmufqYbxJN2Mpuke".to_string(),
            canonical: false,
            was_canonical: true,
        },
        BlockCanonicityUpdatePayload {
            height: 1230,
            state_hash: "3NKNqeqRjSY8Qy1x4qwzfCVSCGUq2czp4Jo9E7pUY6ZnHTBg4JiW".to_string(),
            canonical: false,
            was_canonical: true,
        },
        BlockCanonicityUpdatePayload {
            height: 1231,
            state_hash: "3NKMBzGM1pySPanKkLdxjnUS9mZ88oCeanq3Nhm2QZ6eNB5ZkeGn".to_string(),
            canonical: false,
            was_canonical: true,
        },
        BlockCanonicityUpdatePayload {
            height: 1230,
            state_hash: "3NKcSppgUnmuGp9kGQdp7ZXUh5vcf7rm9mhKE4UMvwcuUaB8wb7S".to_string(),
            canonical: false,
            was_canonical: false,
        },
        BlockCanonicityUpdatePayload {
            height: 1229,
            state_hash: "3NL63pX2kKi4pezYnm6MPjsDK9VSvJ1XFiUaed9vxPEa5PuWPLeZ".to_string(),
            canonical: false,
            was_canonical: false,
        },
        BlockCanonicityUpdatePayload {
            height: 1231,
            state_hash: "3NKMBzGM1pySPanKkLdxjnUS9mZ88oCeanq3Nhm2QZ6eNB5ZkeGn".to_string(),
            canonical: true,
            was_canonical: false,
        },
        BlockCanonicityUpdatePayload {
            height: 1230,
            state_hash: "3NKNqeqRjSY8Qy1x4qwzfCVSCGUq2czp4Jo9E7pUY6ZnHTBg4JiW".to_string(),
            canonical: true,
            was_canonical: false,
        },
        BlockCanonicityUpdatePayload {
            height: 1229,
            state_hash: "3NLU6jZai3xytD6PkpdKNFTYjfYSsEWoksr9AmufqYbxJN2Mpuke".to_string(),
            canonical: true,
            was_canonical: false,
        },
    ];

    while let Some(expected_event) = expected_events.pop() {
        let event = receiver.recv().await;
        let payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&event.unwrap().payload).unwrap();
        assert_eq!(payload.height, expected_event.height);
        assert_eq!(payload.state_hash, expected_event.state_hash);
        assert_eq!(payload.canonical, expected_event.canonical);
        assert_eq!(payload.was_canonical, expected_event.was_canonical);
    }

    Ok(())
}
