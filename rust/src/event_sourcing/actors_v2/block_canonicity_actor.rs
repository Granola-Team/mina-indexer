use crate::{
    blockchain_tree::{BlockchainTree, Hash, Height, Node},
    constants::TRANSITION_FRONTIER_DISTANCE,
    event_sourcing::{
        actor_dag::{ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
        events::{Event, EventType},
        payloads::{BlockCanonicityUpdatePayload, NewBlockPayload},
    },
};
use log::warn;
use std::sync::Arc;
use tokio::sync::{watch, Mutex};

pub struct BlockCanonicityActor;

const BLOCKCHAIN_TREE_KEY: &str = "blockchain_tree";

impl BlockCanonicityActor {
    /// Generate canonical update events for a node
    fn create_canonical_update_events(node: &Node, canonical: bool, was_canonical: bool) -> Vec<Event> {
        let update = BlockCanonicityUpdatePayload {
            height: node.height.0,
            state_hash: node.state_hash.0.clone(),
            canonical,
            was_canonical,
        };

        vec![Event {
            event_type: EventType::BlockCanonicityUpdate,
            payload: sonic_rs::to_string(&update).unwrap(),
        }]
    }

    /// Process blocks of equal height
    fn process_equal_height(blockchain_tree: &mut BlockchainTree, current_best_block: &Node, next_node: &Node) -> Vec<Event> {
        if BlockchainTree::greater(next_node, current_best_block) {
            Self::update_ancestries(blockchain_tree, current_best_block, next_node)
        } else {
            Self::create_canonical_update_events(next_node, false, false)
        }
    }

    /// Process blocks of greater height
    fn process_greater_height(blockchain_tree: &mut BlockchainTree, current_best_block: &Node, next_node: &Node) -> Vec<Event> {
        let parent = blockchain_tree.get_parent(next_node).unwrap();
        let mut events = vec![];

        if parent.state_hash != current_best_block.state_hash {
            events.extend(Self::update_ancestries(blockchain_tree, current_best_block, parent));
        }
        events.extend(Self::create_canonical_update_events(next_node, true, false));
        events
    }

    /// Update ancestries when switching canonical branches
    fn update_ancestries(blockchain_tree: &BlockchainTree, current_best_block: &Node, next_node: &Node) -> Vec<Event> {
        let (prior_ancestry, mut new_ancestry, _) = blockchain_tree.get_shared_ancestry(current_best_block, next_node).unwrap();

        let mut events = vec![];

        // Mark prior ancestry as non-canonical
        for prior in prior_ancestry.iter() {
            events.extend(Self::create_canonical_update_events(prior, false, true));
        }

        // Mark new ancestry as canonical
        new_ancestry.reverse();
        for new_a in new_ancestry.iter() {
            events.extend(Self::create_canonical_update_events(new_a, true, false));
        }

        events
    }
}

impl ActorFactory for BlockCanonicityActor {
    fn create_actor(shutdown_rx: watch::Receiver<bool>) -> ActorNode {
        let mut actor_store = ActorStore::new();
        actor_store.insert(BLOCKCHAIN_TREE_KEY, BlockchainTree::new(TRANSITION_FRONTIER_DISTANCE));

        ActorNodeBuilder::new(EventType::NewBlock)
            .with_state(actor_store)
            .with_processor(|event, state: Arc<Mutex<ActorStore>>, _requeue| {
                Box::pin(async move {
                    let mut state = state.lock().await;
                    let mut blockchain_tree: BlockchainTree = state.remove(BLOCKCHAIN_TREE_KEY).unwrap();

                    // Deserialize the block payload
                    let block_payload: NewBlockPayload = sonic_rs::from_str(&event.payload).unwrap();

                    // Create a node for the blockchain tree
                    let next_node = Node {
                        height: Height(block_payload.height),
                        state_hash: Hash(block_payload.state_hash.clone()),
                        previous_state_hash: Hash(block_payload.previous_state_hash),
                        last_vrf_output: block_payload.last_vrf_output,
                        ..Default::default()
                    };

                    let mut events = vec![];

                    if blockchain_tree.is_empty() {
                        blockchain_tree.set_root(next_node.clone()).unwrap();
                        events.extend(Self::create_canonical_update_events(&next_node, true, false));
                    } else if blockchain_tree.has_parent(&next_node) {
                        let (height, current_best_block) = blockchain_tree.get_best_tip().unwrap();
                        blockchain_tree.add_node(next_node.clone()).unwrap();

                        events.extend(match next_node.height.cmp(&height) {
                            std::cmp::Ordering::Equal => Self::process_equal_height(&mut blockchain_tree, &current_best_block, &next_node),
                            std::cmp::Ordering::Greater => Self::process_greater_height(&mut blockchain_tree, &current_best_block, &next_node),
                            std::cmp::Ordering::Less => Self::create_canonical_update_events(&next_node, false, false),
                        });
                    } else {
                        warn!(
                            "Attempted to add block at height {} and state_hash {} but found no parent",
                            next_node.height.0, next_node.state_hash.0
                        );
                    }

                    // Prune the tree and save state
                    blockchain_tree.prune_tree().unwrap();
                    state.insert(BLOCKCHAIN_TREE_KEY, blockchain_tree);

                    // println!("{:#?}", events);
                    Some(events)
                })
            })
            .build(shutdown_rx)
    }
}

#[cfg(test)]
mod block_canonicity_actor_tests_v2 {
    use super::*;
    use crate::{
        constants::GENESIS_STATE_HASH,
        event_sourcing::{
            actor_dag::{ActorFactory, ActorNode},
            events::{Event, EventType},
            payloads::{BlockCanonicityUpdatePayload, NewBlockPayload},
        },
    };
    use std::sync::Arc;
    use tokio::sync::{watch, Mutex};

    #[tokio::test]
    async fn test_new_block_becomes_canonical_over_existing_block() {
        // Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Create the BlockCanonicityActor
        let mut actor = BlockCanonicityActor::create_actor(shutdown_rx);

        // Add a receiver for capturing BlockCanonicityUpdate events
        let mut receiver = actor.add_receiver(EventType::BlockCanonicityUpdate);

        // Wrap the actor in an Arc<Mutex> for shared ownership
        let actor = Arc::new(Mutex::new(actor));

        // Spawn the actor
        tokio::spawn({
            let actor_clone = Arc::clone(&actor);
            async move {
                ActorNode::spawn_all(actor_clone).await;
            }
        });

        let genesis_block = NewBlockPayload {
            height: 1,
            state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "vrf_output".to_string(),
            previous_state_hash: String::new(),
        };

        // Send the GenesisBlock event to initialize the blockchain tree
        actor
            .lock()
            .await
            .get_sender()
            .unwrap()
            .send(Event {
                event_type: EventType::NewBlock,
                payload: sonic_rs::to_string(&genesis_block).unwrap(),
            })
            .await
            .expect("Failed to send GenesisBlock event");

        if let Ok(Some(received_event)) = tokio::time::timeout(tokio::time::Duration::from_secs(2), receiver.recv()).await {
            assert_eq!(received_event.event_type, EventType::BlockCanonicityUpdate);

            let payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&received_event.payload).unwrap();
            assert_eq!(payload.height, 1);
            assert_eq!(payload.state_hash, GENESIS_STATE_HASH.to_string());
            assert!(payload.canonical);
        } else {
            panic!("Expected initial block to be canonical, but did not receive update.");
        }

        // Define initial and new canonical block payloads
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

        // Send the initial block, which should become canonical
        actor
            .lock()
            .await
            .get_sender()
            .unwrap()
            .send(Event {
                event_type: EventType::NewBlock,
                payload: sonic_rs::to_string(&initial_block_payload).unwrap(),
            })
            .await
            .expect("Failed to send initial block event");

        // Expect the initial block to be marked as canonical
        if let Ok(Some(received_event)) = tokio::time::timeout(tokio::time::Duration::from_secs(2), receiver.recv()).await {
            assert_eq!(received_event.event_type, EventType::BlockCanonicityUpdate);

            let payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&received_event.payload).unwrap();
            assert_eq!(payload.height, 2);
            assert_eq!(payload.state_hash, "initial_block_hash");
            assert!(payload.canonical);
        } else {
            panic!("Expected initial block to be canonical, but did not receive update.");
        }

        // Send the new canonical block, which has a higher VRF output
        actor
            .lock()
            .await
            .get_sender()
            .unwrap()
            .send(Event {
                event_type: EventType::NewBlock,
                payload: sonic_rs::to_string(&new_canonical_block_payload).unwrap(),
            })
            .await
            .expect("Failed to send new canonical block event");

        // Verify that the new block becomes canonical and the old block is marked non-canonical
        let mut received_new_canonical_update = false;
        let mut received_non_canonical_update = false;

        while let Ok(Some(received_event)) = tokio::time::timeout(tokio::time::Duration::from_secs(2), receiver.recv()).await {
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

        // Assert that the updates were received correctly
        assert!(received_new_canonical_update, "New block should be marked as canonical.");
        assert!(received_non_canonical_update, "Initial block should be marked as non-canonical.");

        // Signal shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_non_canonical_block_with_vrf_info() {
        use crate::{
            constants::GENESIS_STATE_HASH,
            event_sourcing::{
                actor_dag::{ActorFactory, ActorNode},
                events::{Event, EventType},
                payloads::{BlockCanonicityUpdatePayload, NewBlockPayload},
            },
        };
        use std::sync::Arc;
        use tokio::sync::{watch, Mutex};

        // Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Create the BlockCanonicityActor
        let mut actor = BlockCanonicityActor::create_actor(shutdown_rx);

        // Add a receiver for capturing BlockCanonicityUpdate events
        let mut receiver = actor.add_receiver(EventType::BlockCanonicityUpdate);

        // Wrap the actor in an Arc<Mutex> for shared ownership
        let actor = Arc::new(Mutex::new(actor));

        // Spawn the actor
        tokio::spawn({
            let actor_clone = Arc::clone(&actor);
            async move {
                ActorNode::spawn_all(actor_clone).await;
            }
        });

        let genesis_block = NewBlockPayload {
            height: 1,
            state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "vrf_output".to_string(),
            previous_state_hash: String::new(),
        };

        // Send the GenesisBlock event to initialize the blockchain tree
        actor
            .lock()
            .await
            .get_sender()
            .unwrap()
            .send(Event {
                event_type: EventType::NewBlock,
                payload: sonic_rs::to_string(&genesis_block).unwrap(),
            })
            .await
            .expect("Failed to send GenesisBlock event");

        if let Ok(Some(received_event)) = tokio::time::timeout(tokio::time::Duration::from_secs(2), receiver.recv()).await {
            assert_eq!(received_event.event_type, EventType::BlockCanonicityUpdate);

            let payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&received_event.payload).unwrap();
            assert_eq!(payload.height, 1);
            assert_eq!(payload.state_hash, GENESIS_STATE_HASH.to_string());
            assert!(payload.canonical);
        } else {
            panic!("Expected Genesis block to be canonical, but did not receive update.");
        }

        // Define canonical and non-canonical block payloads
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

        // Send the canonical block, which should become canonical
        actor
            .lock()
            .await
            .get_sender()
            .unwrap()
            .send(Event {
                event_type: EventType::NewBlock,
                payload: sonic_rs::to_string(&canonical_block_payload).unwrap(),
            })
            .await
            .expect("Failed to send canonical block event");

        // Expect the canonical block to be marked as canonical
        if let Ok(Some(received_event)) = tokio::time::timeout(tokio::time::Duration::from_secs(2), receiver.recv()).await {
            assert_eq!(received_event.event_type, EventType::BlockCanonicityUpdate);

            let payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&received_event.payload).unwrap();
            assert_eq!(payload.height, 2);
            assert_eq!(payload.state_hash, "canonical_hash");
            assert!(payload.canonical);
        } else {
            panic!("Expected a BlockCanonicityUpdate event for canonical block but did not receive one.");
        }

        // Send the non-canonical block
        actor
            .lock()
            .await
            .get_sender()
            .unwrap()
            .send(Event {
                event_type: EventType::NewBlock,
                payload: sonic_rs::to_string(&non_canonical_block_payload).unwrap(),
            })
            .await
            .expect("Failed to send non-canonical block event");

        // Expect the non-canonical block to be marked as non-canonical
        if let Ok(Some(received_event)) = tokio::time::timeout(tokio::time::Duration::from_secs(2), receiver.recv()).await {
            assert_eq!(received_event.event_type, EventType::BlockCanonicityUpdate);

            let payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&received_event.payload).unwrap();
            assert_eq!(payload.height, 2);
            assert_eq!(payload.state_hash, "non_canonical_hash");
            assert!(!payload.canonical);
        } else {
            panic!("Expected a BlockCanonicityUpdate event for non-canonical block but did not receive one.");
        }

        // Signal shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_longer_branch_outcompetes_canonical_branch_with_tiebreaker() {
        use crate::{
            constants::GENESIS_STATE_HASH,
            event_sourcing::{
                actor_dag::{ActorFactory, ActorNode},
                events::{Event, EventType},
                payloads::{BlockCanonicityUpdatePayload, NewBlockPayload},
            },
        };
        use std::sync::Arc;
        use tokio::sync::{watch, Mutex};

        // Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Create the BlockCanonicityActor
        let mut actor = BlockCanonicityActor::create_actor(shutdown_rx);

        // Add a receiver for capturing BlockCanonicityUpdate events
        let mut receiver = actor.add_receiver(EventType::BlockCanonicityUpdate);

        // Wrap the actor in an Arc<Mutex> for shared ownership
        let actor = Arc::new(Mutex::new(actor));

        // Spawn the actor
        tokio::spawn({
            let actor_clone = Arc::clone(&actor);
            async move {
                ActorNode::spawn_all(actor_clone).await;
            }
        });

        let genesis_block = NewBlockPayload {
            height: 1,
            state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "vrf_output".to_string(),
            previous_state_hash: String::new(),
        };

        // Send the GenesisBlock event to initialize the blockchain tree
        actor
            .lock()
            .await
            .get_sender()
            .unwrap()
            .send(Event {
                event_type: EventType::NewBlock,
                payload: sonic_rs::to_string(&genesis_block).unwrap(),
            })
            .await
            .expect("Failed to send GenesisBlock event");

        // Discard the genesis block for this test
        let _ = receiver.recv().await.expect("Expected event not received");

        // Define block payloads
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

        // Send original branch blocks
        for payload in [&original_block1_payload, &original_block2_payload] {
            actor
                .lock()
                .await
                .get_sender()
                .unwrap()
                .send(Event {
                    event_type: EventType::NewBlock,
                    payload: sonic_rs::to_string(payload).unwrap(),
                })
                .await
                .expect("Failed to send original branch block event");
        }

        // Send competing branch blocks
        for payload in [&competing_block1_payload, &competing_block2_payload, &competing_block3_payload] {
            actor
                .lock()
                .await
                .get_sender()
                .unwrap()
                .send(Event {
                    event_type: EventType::NewBlock,
                    payload: sonic_rs::to_string(payload).unwrap(),
                })
                .await
                .expect("Failed to send competing branch block event");
        }

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
        for expected_event in expected_events.into_iter() {
            let received_event = receiver.recv().await.expect("Expected event not received");
            assert_eq!(received_event.event_type, EventType::BlockCanonicityUpdate);

            let payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&received_event.payload).unwrap();
            assert_eq!(payload, expected_event);
        }

        // Signal shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_block_with_different_parent_at_same_height() {
        use crate::{
            constants::GENESIS_STATE_HASH,
            event_sourcing::{
                actor_dag::{ActorFactory, ActorNode},
                events::{Event, EventType},
                payloads::{BlockCanonicityUpdatePayload, NewBlockPayload},
            },
        };
        use std::sync::Arc;
        use tokio::sync::{watch, Mutex};

        // Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Create the BlockCanonicityActor
        let mut actor = BlockCanonicityActor::create_actor(shutdown_rx);

        // Add a receiver for capturing BlockCanonicityUpdate events
        let mut receiver = actor.add_receiver(EventType::BlockCanonicityUpdate);

        // Wrap the actor in an Arc<Mutex> for shared ownership
        let actor = Arc::new(Mutex::new(actor));

        // Spawn the actor
        tokio::spawn({
            let actor_clone = Arc::clone(&actor);
            async move {
                ActorNode::spawn_all(actor_clone).await;
            }
        });

        // Define root block payload
        let root_block = NewBlockPayload {
            height: 1228,
            state_hash: "3NLGaCf4zNXouyKDkmrFuYBVt8BHnCuLC9PMvkytfFgiuuqv5xsH".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "QNWhrsZ3Ld1gsJTUGBUBCs9iQD00hKUvZkR1GeFjEgA=".to_string(),
        };

        // Define branch 1 blocks
        let branch1_blocks = vec![
            NewBlockPayload {
                height: 1229,
                state_hash: "3NLU6jZai3xytD6PkpdKNFTYjfYSsEWoksr9AmufqYbxJN2Mpuke".to_string(),
                previous_state_hash: root_block.state_hash.clone(),
                last_vrf_output: "PhNdMGQWACdnDnOg3D75icmHf5Mu_0F44ua-fzz4DwA=".to_string(),
            },
            NewBlockPayload {
                height: 1230,
                state_hash: "3NKNqeqRjSY8Qy1x4qwzfCVSCGUq2czp4Jo9E7pUY6ZnHTBg4JiW".to_string(),
                previous_state_hash: "3NLU6jZai3xytD6PkpdKNFTYjfYSsEWoksr9AmufqYbxJN2Mpuke".to_string(),
                last_vrf_output: "h8fO60ijmmBdiMfCBSPz47vsRW8BHg2hnYo4iwl3BAA=".to_string(),
            },
            NewBlockPayload {
                height: 1231,
                state_hash: "3NKMBzGM1pySPanKkLdxjnUS9mZ88oCeanq3Nhm2QZ6eNB5ZkeGn".to_string(),
                previous_state_hash: "3NKNqeqRjSY8Qy1x4qwzfCVSCGUq2czp4Jo9E7pUY6ZnHTBg4JiW".to_string(),
                last_vrf_output: "h8fO60ijmmBdiMfCBSPz47vsRW8BHg2hnYo4iwl3BAA=".to_string(),
            },
        ];

        // Define branch 2 blocks
        let branch2_blocks = vec![
            NewBlockPayload {
                height: 1229,
                state_hash: "3NL63pX2kKi4pezYnm6MPjsDK9VSvJ1XFiUaed9vxPEa5PuWPLeZ".to_string(),
                previous_state_hash: root_block.state_hash.clone(),
                last_vrf_output: "hPuL8ZcZI2pIZhbVLVVD0U4CO0FLlnIgW_rYULV_HAA=".to_string(),
            },
            NewBlockPayload {
                height: 1230,
                state_hash: "3NKcSppgUnmuGp9kGQdp7ZXUh5vcf7rm9mhKE4UMvwcuUaB8wb7S".to_string(),
                previous_state_hash: "3NL63pX2kKi4pezYnm6MPjsDK9VSvJ1XFiUaed9vxPEa5PuWPLeZ".to_string(),
                last_vrf_output: "Wl4EiJDCuzNCCn2w_6vDywRZStf4iNM5h4qUxXvOFAA=".to_string(),
            },
            NewBlockPayload {
                height: 1231,
                state_hash: "3NLN2uxCoRsiB2C4uZHDcY6WxJDz6EEWmoruCFva74E4cgTYgJCQ".to_string(),
                previous_state_hash: "3NKcSppgUnmuGp9kGQdp7ZXUh5vcf7rm9mhKE4UMvwcuUaB8wb7S".to_string(),
                last_vrf_output: "qQNTDpNokYfZs1wlQNcSLWazVCu43O7mLAhxlgwjBAA=".to_string(),
            },
        ];

        // Send the root block
        actor
            .lock()
            .await
            .get_sender()
            .unwrap()
            .send(Event {
                event_type: EventType::NewBlock,
                payload: sonic_rs::to_string(&root_block).unwrap(),
            })
            .await
            .expect("Failed to send root block");

        // Discard the root block event
        let _ = receiver.recv().await.expect("Expected event not received");

        // Send branch 1 blocks
        for block in &branch1_blocks {
            actor
                .lock()
                .await
                .get_sender()
                .unwrap()
                .send(Event {
                    event_type: EventType::NewBlock,
                    payload: sonic_rs::to_string(block).unwrap(),
                })
                .await
                .expect("Failed to send branch 1 block event");
        }

        // Send branch 2 blocks
        for block in &branch2_blocks {
            actor
                .lock()
                .await
                .get_sender()
                .unwrap()
                .send(Event {
                    event_type: EventType::NewBlock,
                    payload: sonic_rs::to_string(block).unwrap(),
                })
                .await
                .expect("Failed to send branch 2 block event");
        }

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
            let received_event = receiver.recv().await.expect("Expected event not received");
            assert_eq!(received_event.event_type, EventType::BlockCanonicityUpdate);

            let payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&received_event.payload).unwrap();
            assert_eq!(payload.height, expected_event.height);
            assert_eq!(payload.state_hash, expected_event.state_hash);
            assert_eq!(payload.canonical, expected_event.canonical);
            assert_eq!(payload.was_canonical, expected_event.was_canonical);
        }

        // Signal shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }
}
