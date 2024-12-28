use crate::{
    blockchain_tree::{BlockchainTree, Hash, Height, Node},
    constants::TRANSITION_FRONTIER_DISTANCE,
    event_sourcing::{
        actor_dag::{ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
        events::{Event, EventType},
        payloads::{BlockCanonicityUpdatePayload, NewBlockPayload},
    },
};
use async_trait::async_trait;
use log::warn;
use std::sync::Arc;
use tokio::sync::Mutex;

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

#[async_trait]
impl ActorFactory for BlockCanonicityActor {
    async fn create_actor() -> ActorNode {
        let mut actor_store = ActorStore::new();
        actor_store.insert(BLOCKCHAIN_TREE_KEY, BlockchainTree::new(TRANSITION_FRONTIER_DISTANCE));

        ActorNodeBuilder::new()
            .with_state(actor_store)
            .with_processor(|event, state: Arc<Mutex<ActorStore>>, _requeue| {
                Box::pin(async move {
                    if event.event_type == EventType::NewBlock {
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

                        Some(events)
                    } else {
                        None
                    }
                })
            })
            .build()
    }
}

#[cfg(test)]
mod block_canonicity_actor_tests_v2 {
    use super::BlockCanonicityActor;
    use crate::{
        constants::GENESIS_STATE_HASH,
        event_sourcing::{
            actor_dag::{ActorDAG, ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
            events::{Event, EventType},
            payloads::{BlockCanonicityUpdatePayload, NewBlockPayload},
        },
    };
    use sonic_rs;
    use std::sync::Arc;
    use tokio::{
        sync::{watch, Mutex},
        time::{sleep, Duration},
    };

    // ---------------------------------------------------------
    // SINK NODE + HELPER FUNCTIONS
    // ---------------------------------------------------------

    /// Creates a sink node to capture all `BlockCanonicityUpdate` events.
    fn create_canonicity_sink_node() -> ActorNode {
        ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|event, state, _requeue| {
                Box::pin(async move {
                    if event.event_type == EventType::BlockCanonicityUpdate {
                        let mut locked_state = state.lock().await;
                        // Keep a vector of all canonicity updates
                        let mut captured_updates: Vec<String> = locked_state.get("captured_canonicity_updates").cloned().unwrap_or_default();
                        captured_updates.push(event.payload.clone());
                        locked_state.insert("captured_canonicity_updates", captured_updates);
                    }
                    None
                })
            })
            .build()
    }

    /// Reads all `BlockCanonicityUpdate` payloads (as strings) from the sink node's state.
    async fn read_canonicity_updates(dag: &Arc<Mutex<ActorDAG>>, sink_node_id: &str) -> Vec<String> {
        let dag_locked = dag.lock().await;
        let sink_node_locked = dag_locked.read_node(sink_node_id.to_string()).expect("Sink node not found").lock().await;
        let state = sink_node_locked.get_state();
        let store_locked = state.lock().await;

        store_locked.get::<Vec<String>>("captured_canonicity_updates").cloned().unwrap_or_default()
    }

    // ---------------------------------------------------------
    // TESTS
    // ---------------------------------------------------------

    #[tokio::test]
    async fn test_new_block_becomes_canonical_over_existing_block() {
        // 1. Create the shutdown signal
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2. Create an ActorDAG
        let mut dag = ActorDAG::new();

        // 3. Create the BlockCanonicityActor node (root)
        let canonicity_actor = BlockCanonicityActor::create_actor().await;
        let canonicity_actor_id = canonicity_actor.id();

        // 4. Set the root in the DAG to get a `Sender<Event>`
        let canonicity_sender = dag.set_root(canonicity_actor);

        // 5. Create a sink node to capture `BlockCanonicityUpdate` events
        let sink_node = create_canonicity_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&canonicity_actor_id, sink_node_id);

        // 6. Wrap the DAG in Arc<Mutex<>> and spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 7. Send the genesis block
        let genesis_block = NewBlockPayload {
            height: 1,
            state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "vrf_output".to_string(),
            previous_state_hash: String::new(),
        };
        canonicity_sender
            .send(Event {
                event_type: EventType::NewBlock,
                payload: sonic_rs::to_string(&genesis_block).unwrap(),
            })
            .await
            .expect("Failed to send GenesisBlock event");
        sleep(Duration::from_millis(100)).await;

        // Check that the genesis block was marked canonical
        let updates = read_canonicity_updates(&dag, sink_node_id).await;
        assert_eq!(updates.len(), 1, "Expected 1 canonicity update for genesis");

        let genesis_update: BlockCanonicityUpdatePayload = sonic_rs::from_str(&updates[0]).expect("Failed to parse genesis update");
        assert_eq!(genesis_update.height, 1);
        assert_eq!(genesis_update.state_hash, GENESIS_STATE_HASH.to_string());
        assert!(genesis_update.canonical);

        // 8. Send the initial block (height=2) which becomes canonical
        let initial_block_payload = NewBlockPayload {
            height: 2,
            state_hash: "initial_block_hash".to_string(),
            last_vrf_output: "a_vrf_output".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
        };
        canonicity_sender
            .send(Event {
                event_type: EventType::NewBlock,
                payload: sonic_rs::to_string(&initial_block_payload).unwrap(),
            })
            .await
            .expect("Failed to send initial block event");
        sleep(Duration::from_millis(100)).await;

        let updates = read_canonicity_updates(&dag, sink_node_id).await;
        // We expect 2 total updates now: genesis + newly canonical block
        assert_eq!(updates.len(), 2, "Expected 2 total canonicity updates");

        let update: BlockCanonicityUpdatePayload = sonic_rs::from_str(&updates[1]).expect("Failed to parse initial block update");
        assert_eq!(update.height, 2);
        assert_eq!(update.state_hash, "initial_block_hash");
        assert!(update.canonical);

        // 9. Send a new block with the same height but a better (higher) VRF output
        let new_canonical_block_payload = NewBlockPayload {
            height: 2,
            state_hash: "new_canonical_hash".to_string(),
            last_vrf_output: "b_vrf_output".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
        };
        canonicity_sender
            .send(Event {
                event_type: EventType::NewBlock,
                payload: sonic_rs::to_string(&new_canonical_block_payload).unwrap(),
            })
            .await
            .expect("Failed to send new canonical block event");
        sleep(Duration::from_millis(100)).await;

        // Expect the new block to be canonical and old block to be non-canonical
        let updates = read_canonicity_updates(&dag, sink_node_id).await;
        // We had 2 from before + 2 more for the shift in canonicity
        assert_eq!(updates.len(), 4, "Expected 4 total canonicity updates");

        let last_two = &updates[2..4];
        let mut received_new_canonical = false;
        let mut received_old_non_canonical = false;

        for upd in last_two {
            let payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(upd).expect("Failed to parse canonicity update");
            match payload.state_hash.as_str() {
                "new_canonical_hash" => {
                    assert!(payload.canonical);
                    received_new_canonical = true;
                }
                "initial_block_hash" => {
                    assert!(!payload.canonical);
                    received_old_non_canonical = true;
                }
                _ => {}
            }
        }

        assert!(received_new_canonical, "New block should be canonical");
        assert!(received_old_non_canonical, "Initial block should be non-canonical");

        // 10. Shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_non_canonical_block_with_vrf_info() {
        // 1. Create the shutdown signal
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2. Create an ActorDAG
        let mut dag = ActorDAG::new();

        // 3. Create BlockCanonicityActor, set as root
        let canonicity_actor = BlockCanonicityActor::create_actor().await;
        let canonicity_actor_id = canonicity_actor.id();
        let canonicity_sender = dag.set_root(canonicity_actor);

        // 4. Create a sink node
        let sink_node = create_canonicity_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&canonicity_actor_id, sink_node_id);

        // 5. Wrap + spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 6. Send the genesis block
        let genesis_block = NewBlockPayload {
            height: 1,
            state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "vrf_output".to_string(),
            previous_state_hash: String::new(),
        };
        canonicity_sender
            .send(Event {
                event_type: EventType::NewBlock,
                payload: sonic_rs::to_string(&genesis_block).unwrap(),
            })
            .await
            .expect("Failed to send GenesisBlock");
        sleep(Duration::from_millis(100)).await;

        let updates = read_canonicity_updates(&dag, sink_node_id).await;
        assert_eq!(updates.len(), 1, "Genesis block should be canonical");

        // 7. Define canonical and non-canonical blocks
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

        // 8. Send the canonical block
        canonicity_sender
            .send(Event {
                event_type: EventType::NewBlock,
                payload: sonic_rs::to_string(&canonical_block_payload).unwrap(),
            })
            .await
            .expect("Failed to send canonical block");
        sleep(Duration::from_millis(100)).await;

        let updates = read_canonicity_updates(&dag, sink_node_id).await;
        assert_eq!(updates.len(), 2, "We now have genesis + canonical block updates");
        let canonical_update: BlockCanonicityUpdatePayload = sonic_rs::from_str(&updates[1]).expect("Failed to parse canonical update");
        assert_eq!(canonical_update.state_hash, "canonical_hash");
        assert!(canonical_update.canonical);

        // 9. Send the non-canonical block
        canonicity_sender
            .send(Event {
                event_type: EventType::NewBlock,
                payload: sonic_rs::to_string(&non_canonical_block_payload).unwrap(),
            })
            .await
            .expect("Failed to send non-canonical block");
        sleep(Duration::from_millis(100)).await;

        let updates = read_canonicity_updates(&dag, sink_node_id).await;
        assert_eq!(updates.len(), 3, "One more update for the non-canonical block");
        let last_update: BlockCanonicityUpdatePayload = sonic_rs::from_str(&updates[2]).expect("Failed to parse last update");
        assert_eq!(last_update.state_hash, "non_canonical_hash");
        assert!(!last_update.canonical);

        // 10. Shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_longer_branch_outcompetes_canonical_branch_with_tiebreaker() {
        // 1. Create the shutdown signal
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2. Create an ActorDAG
        let mut dag = ActorDAG::new();

        // 3. Create the BlockCanonicityActor node (root)
        let canonicity_actor = BlockCanonicityActor::create_actor().await;
        let canonicity_actor_id = canonicity_actor.id();

        // 4. Set the root in the DAG to get a `Sender<Event>`
        let canonicity_sender = dag.set_root(canonicity_actor);

        // 5. Create a sink node to capture `BlockCanonicityUpdate` events
        let sink_node = create_canonicity_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&canonicity_actor_id, sink_node_id);

        // 6. Wrap the DAG and spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 7. Send the genesis block
        let genesis_block = NewBlockPayload {
            height: 1,
            state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "vrf_output".to_string(),
            previous_state_hash: String::new(),
        };
        canonicity_sender
            .send(Event {
                event_type: EventType::NewBlock,
                payload: sonic_rs::to_string(&genesis_block).unwrap(),
            })
            .await
            .expect("Failed to send genesis block");
        sleep(Duration::from_millis(100)).await;

        // We discard the genesis block's canonicity update
        let updates_after_genesis = read_canonicity_updates(&dag, sink_node_id).await;
        if updates_after_genesis.is_empty() {
            panic!("Expected genesis block to be marked canonical, but got no updates.");
        }

        // 8. Define blocks for the original branch
        let original_block1 = NewBlockPayload {
            height: 2,
            state_hash: "original_block_1".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "b_vrf_output".to_string(),
        };
        let original_block2 = NewBlockPayload {
            height: 3,
            state_hash: "original_block_2".to_string(),
            previous_state_hash: "original_block_1".to_string(),
            last_vrf_output: "b_vrf_output".to_string(),
        };

        // 9. Define blocks for the competing branch
        let competing_block1 = NewBlockPayload {
            height: 2,
            state_hash: "competing_block_1".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "a_vrf_output".to_string(),
        };
        let competing_block2 = NewBlockPayload {
            height: 3,
            state_hash: "competing_block_2".to_string(),
            previous_state_hash: "competing_block_1".to_string(),
            last_vrf_output: "a_vrf_output".to_string(),
        };
        let competing_block3 = NewBlockPayload {
            height: 4,
            state_hash: "competing_block_3".to_string(),
            previous_state_hash: "competing_block_2".to_string(),
            last_vrf_output: "a_vrf_output".to_string(),
        };

        // 10. Send the original branch blocks
        for block in &[original_block1, original_block2] {
            canonicity_sender
                .send(Event {
                    event_type: EventType::NewBlock,
                    payload: sonic_rs::to_string(block).unwrap(),
                })
                .await
                .expect("Failed to send original branch block");
            sleep(Duration::from_millis(100)).await;
        }

        // 11. Send the competing branch blocks
        for block in &[competing_block1, competing_block2, competing_block3] {
            canonicity_sender
                .send(Event {
                    event_type: EventType::NewBlock,
                    payload: sonic_rs::to_string(block).unwrap(),
                })
                .await
                .expect("Failed to send competing branch block");
            sleep(Duration::from_millis(100)).await;
        }

        // 12. Expected canonicity updates (in the order they should arrive)
        // These match the original logic: original blocks become canonical, then competing blocks
        // override them. We have VRF tiebreakers + longer branch, etc.
        let expected_events = vec![
            // Original block1 canonical
            BlockCanonicityUpdatePayload {
                height: 2,
                state_hash: "original_block_1".to_string(),
                canonical: true,
                was_canonical: false,
            },
            // Original block2 canonical
            BlockCanonicityUpdatePayload {
                height: 3,
                state_hash: "original_block_2".to_string(),
                canonical: true,
                was_canonical: false,
            },
            // Competing block1 non-canonical
            BlockCanonicityUpdatePayload {
                height: 2,
                state_hash: "competing_block_1".to_string(),
                canonical: false,
                was_canonical: false,
            },
            // Competing block2 non-canonical
            BlockCanonicityUpdatePayload {
                height: 3,
                state_hash: "competing_block_2".to_string(),
                canonical: false,
                was_canonical: false,
            },
            // Now the competing branch outcompetes: original_block_2 becomes non-canonical
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
            // Finally, competing_block_3 arrives, also canonical
            BlockCanonicityUpdatePayload {
                height: 4,
                state_hash: "competing_block_3".to_string(),
                canonical: true,
                was_canonical: false,
            },
        ];

        // 13. Read all updates
        let all_updates = read_canonicity_updates(&dag, sink_node_id).await;

        // We expect the genesis block was 1 update, so total = 1 + expected_events.len()
        // We skip the genesis update by ignoring the first.
        if all_updates.len() != expected_events.len() + 1 {
            panic!(
                "Expected {} total updates (including genesis), but got {}",
                expected_events.len() + 1,
                all_updates.len()
            );
        }

        // The first is genesis, so skip it
        let relevant_updates = &all_updates[1..];

        // Match each expected event to the corresponding update in order
        for (i, expected) in expected_events.into_iter().enumerate() {
            let update_json = &relevant_updates[i];
            let parsed: BlockCanonicityUpdatePayload = sonic_rs::from_str(update_json).expect("Failed to parse canonicity update");
            if parsed != expected {
                panic!("Mismatch at index {}. \nExpected: {:?}\nGot: {:?}", i, expected, parsed);
            }
        }

        // 14. Shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_block_with_different_parent_at_same_height() {
        // 1. Create the shutdown signal
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2. Create an ActorDAG
        let mut dag = ActorDAG::new();

        // 3. Create BlockCanonicityActor, set as root
        let canonicity_actor = BlockCanonicityActor::create_actor().await;
        let canonicity_actor_id = canonicity_actor.id();
        let canonicity_sender = dag.set_root(canonicity_actor);

        // 4. Create a sink node
        let sink_node = create_canonicity_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&canonicity_actor_id, sink_node_id);

        // 5. Wrap + spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 6. Send a "root" block at height=1228
        let root_block = NewBlockPayload {
            height: 1228,
            state_hash: "3NLGaCf4zNXouyKDkmrFuYBVt8BHnCuLC9PMvkytfFgiuuqv5xsH".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "QNWhrsZ3Ld1gsJTUGBUBCs9iQD00hKUvZkR1GeFjEgA=".to_string(),
        };
        canonicity_sender
            .send(Event {
                event_type: EventType::NewBlock,
                payload: sonic_rs::to_string(&root_block).unwrap(),
            })
            .await
            .expect("Failed to send root block");

        tokio::time::sleep(Duration::from_millis(100)).await;

        // 7. Discard the single update for the root block We read one event from the sink node and do not compare it to anything because it only marks the root
        //    block as canonical.
        if tokio::time::timeout(Duration::from_millis(500), async { read_canonicity_updates(&dag, sink_node_id).await.pop() })
            .await
            .expect("Failed to read sink node updates for root block")
            .is_none()
        {
            panic!("Expected a canonicity update for the root block, but got none.");
        }

        // 8. Define branch1 blocks
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

        // 9. Define branch2 blocks
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

        // 10. Send branch1 blocks
        for block in &branch1_blocks {
            canonicity_sender
                .send(Event {
                    event_type: EventType::NewBlock,
                    payload: sonic_rs::to_string(block).unwrap(),
                })
                .await
                .expect("Failed to send branch1 block");
            sleep(Duration::from_millis(80)).await;
        }

        // 11. Send branch2 blocks
        for block in &branch2_blocks {
            canonicity_sender
                .send(Event {
                    event_type: EventType::NewBlock,
                    payload: sonic_rs::to_string(block).unwrap(),
                })
                .await
                .expect("Failed to send branch2 block");
            sleep(Duration::from_millis(80)).await;
        }

        sleep(Duration::from_millis(100)).await;

        // 12. We now expect a certain sequence of canonicity updates in this exact order:
        let mut expected_updates = vec![
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
            BlockCanonicityUpdatePayload {
                height: 1228,
                state_hash: "3NLGaCf4zNXouyKDkmrFuYBVt8BHnCuLC9PMvkytfFgiuuqv5xsH".to_string(),
                canonical: true,
                was_canonical: false,
            },
        ];

        // 13. We'll read events one-by-one, matching them in the order above.
        let mut i = 0;
        while let Some(expected_update) = expected_updates.pop() {
            let next_update_json = tokio::time::timeout(Duration::from_secs(2), async {
                let updates = read_canonicity_updates(&dag, sink_node_id).await;
                Some(updates[i].clone())
            })
            .await
            .expect("Timed out waiting for next canonicity update");
            let next_update_json = match next_update_json {
                Some(u) => u,
                None => panic!("Expected {:#?}, but no more updates were available.", expected_update),
            };

            let parsed: BlockCanonicityUpdatePayload = sonic_rs::from_str(&next_update_json).expect("Failed to parse canonicity update");
            if parsed != expected_update {
                panic!("Mismatch at {i}.\nExpected: {:#?}\nGot: {:#?}", expected_update, parsed);
            }
            i += 1;
        }

        // 14. Shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }
}
