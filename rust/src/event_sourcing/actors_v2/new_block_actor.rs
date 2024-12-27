use crate::{
    blockchain_tree::{BlockchainTree, Hash, Height, Node},
    constants::TRANSITION_FRONTIER_DISTANCE,
    event_sourcing::{
        actor_dag::{ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
        events::{Event, EventType},
        payloads::BlockAncestorPayload,
    },
};
use log::warn;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct NewBlockActor;

const BLOCKCHAIN_TREE_KEY: &str = "blockchain_tree";

impl ActorFactory for NewBlockActor {
    fn create_actor() -> ActorNode {
        let mut state = ActorStore::new();
        state.insert("blockchain_tree", BlockchainTree::new(TRANSITION_FRONTIER_DISTANCE));
        ActorNodeBuilder::new("NewBlockActor".to_string())
            .with_state(state)
            .with_processor(|event, state: Arc<Mutex<ActorStore>>, requeue| {
                Box::pin(async move {
                    if event.event_type == EventType::BlockAncestor {
                        let mut state = state.lock().await;
                        let mut blockchain_tree: BlockchainTree = state.remove(BLOCKCHAIN_TREE_KEY).unwrap();

                        let block_payload: BlockAncestorPayload = sonic_rs::from_str(&event.payload).unwrap();
                        let next_node = Node {
                            height: Height(block_payload.height),
                            state_hash: Hash(block_payload.state_hash.clone()),
                            previous_state_hash: Hash(block_payload.previous_state_hash.clone()),
                            last_vrf_output: block_payload.last_vrf_output.clone(),
                            ..Default::default()
                        };

                        if blockchain_tree.is_empty() {
                            blockchain_tree.set_root(next_node.clone()).unwrap();
                        } else if blockchain_tree.has_parent(&next_node) {
                            blockchain_tree.add_node(next_node).unwrap();
                        } else {
                            if let Err(err) = requeue.send(event).await {
                                warn!("Unable to requeue event: {err}");
                            }
                            state.insert(BLOCKCHAIN_TREE_KEY, blockchain_tree);
                            return None;
                        }

                        // Publish the BlockAncestor event
                        let added_payload = BlockAncestorPayload {
                            height: block_payload.height,
                            state_hash: block_payload.state_hash,
                            previous_state_hash: block_payload.previous_state_hash,
                            last_vrf_output: block_payload.last_vrf_output,
                        };

                        blockchain_tree.prune_tree().unwrap();

                        state.insert(BLOCKCHAIN_TREE_KEY, blockchain_tree);

                        Some(vec![Event {
                            event_type: EventType::NewBlock,
                            payload: sonic_rs::to_string(&added_payload).unwrap(),
                        }])
                    } else {
                        None
                    }
                })
            })
            .build()
    }
}

#[cfg(test)]
mod blockchain_tree_builder_actor_tests_v2 {
    use super::NewBlockActor;
    use crate::event_sourcing::{
        actor_dag::{ActorDAG, ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
        events::{Event, EventType},
        payloads::BlockAncestorPayload,
    };
    use sonic_rs;
    use std::sync::Arc;
    use tokio::{
        sync::{watch, Mutex},
        time::{sleep, Duration},
    };

    /// Helper function to create a sink node that captures `BlockAncestor` events.
    fn create_new_block_sink_node(id: &str) -> impl FnOnce() -> ActorNode {
        let sink_node_id = id.to_string();
        move || {
            ActorNodeBuilder::new(sink_node_id)
                .with_state(ActorStore::new())
                .with_processor(|event, state, _requeue| {
                    Box::pin(async move {
                        if event.event_type == EventType::NewBlock {
                            // Store all `BlockAncestor` payloads in a vector.
                            let mut locked_state = state.lock().await;
                            let mut captured_blocks: Vec<String> = locked_state.get("captured_blocks").cloned().unwrap_or_default();
                            captured_blocks.push(event.payload.clone());
                            locked_state.insert("captured_blocks", captured_blocks);
                        }
                        None
                    })
                })
                .build()
        }
    }

    /// Reads and returns all captured `BlockAncestor` payloads from the sink node state.
    async fn read_new_block_payloads(dag: &Arc<Mutex<ActorDAG>>, sink_node_id: &str) -> Vec<String> {
        let dag_locked = dag.lock().await;
        let sink_node_locked = dag_locked.read_node(sink_node_id.to_string()).expect("Sink node not found").lock().await;

        let state = sink_node_locked.get_state();
        let store_locked = state.lock().await;
        store_locked.get::<Vec<String>>("captured_blocks").cloned().unwrap_or_default()
    }

    #[tokio::test]
    async fn test_add_root_to_empty_tree() {
        // 1. Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // 2. Create the DAG
        let mut dag = ActorDAG::new();

        // 3. Create your BlockAncestorActor node (root)
        let new_block_actor = NewBlockActor::create_actor();
        let new_block_actor_id = new_block_actor.id();

        // 4. Add it as root, which returns a `Sender<Event>`
        let new_block_sender = dag.set_root(new_block_actor);

        // 5. Create a sink node to capture `BlockAncestor` events
        let sink_node_id = "BlockAncestorSinkRootTest".to_string();
        let sink_node = create_new_block_sink_node(&sink_node_id)();

        // 6. Add the sink node and link it to the actor
        dag.add_node(sink_node);
        dag.link_parent(&new_block_actor_id, &sink_node_id);

        // 7. Wrap the DAG in an `Arc<Mutex<>>`
        let dag = Arc::new(Mutex::new(dag));

        // 8. Spawn the DAG
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all(shutdown_rx).await;
            }
        });

        // 9. Create and send a BlockAncestor event (the "root" block)
        let root_payload = BlockAncestorPayload {
            height: 1,
            state_hash: "3N8aRootHash".to_string(),
            previous_state_hash: "".to_string(),
            last_vrf_output: "".to_string(),
        };
        let root_event = Event {
            event_type: EventType::BlockAncestor,
            payload: sonic_rs::to_string(&root_payload).unwrap(),
        };
        new_block_sender.send(root_event).await.expect("Failed to send root block");

        // 10. Allow some processing time
        sleep(Duration::from_millis(100)).await;

        // 11. Read the sink node's captured blocks
        let captured_blocks = read_new_block_payloads(&dag, &sink_node_id).await;
        assert_eq!(captured_blocks.len(), 1, "Should have 1 BlockAncestor event");

        // 12. Deserialize and verify the BlockAncestorPayload
        let new_block_payload: BlockAncestorPayload = sonic_rs::from_str(&captured_blocks[0]).expect("Failed to parse BlockAncestorPayload");
        assert_eq!(new_block_payload.height, 1);
        assert_eq!(new_block_payload.state_hash, "3N8aRootHash");

        // 13. Shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_add_node_with_existing_parent() {
        // 1. Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // 2. Create the DAG
        let mut dag = ActorDAG::new();

        // 3. Create your BlockAncestorActor node (root)
        let new_block_actor = NewBlockActor::create_actor();
        let new_block_actor_id = new_block_actor.id();

        // 4. Add it as root to get a `Sender<Event>`
        let new_block_sender = dag.set_root(new_block_actor);

        // 5. Create a sink node
        let sink_node_id = "BlockAncestorSinkChildTest".to_string();
        let sink_node = create_new_block_sink_node(&sink_node_id)();
        dag.add_node(sink_node);
        dag.link_parent(&new_block_actor_id, &sink_node_id);

        // 6. Wrap and spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all(shutdown_rx).await;
            }
        });

        // 7. Send the root block
        let root_payload = BlockAncestorPayload {
            height: 1,
            state_hash: "3N8aRootHash".to_string(),
            previous_state_hash: "".to_string(),
            last_vrf_output: "".to_string(),
        };
        new_block_sender
            .send(Event {
                event_type: EventType::BlockAncestor,
                payload: sonic_rs::to_string(&root_payload).unwrap(),
            })
            .await
            .expect("Failed to send root block");
        sleep(Duration::from_millis(100)).await;

        // 8. Send a child block that references the root
        let child_payload = BlockAncestorPayload {
            height: 2,
            state_hash: "3N8aChildHash".to_string(),
            previous_state_hash: "3N8aRootHash".to_string(),
            last_vrf_output: "".to_string(),
        };
        new_block_sender
            .send(Event {
                event_type: EventType::BlockAncestor,
                payload: sonic_rs::to_string(&child_payload).unwrap(),
            })
            .await
            .expect("Failed to send child block");
        sleep(Duration::from_millis(100)).await;

        // 9. Now we expect two captured BlockAncestor payloads in order: height=1, then height=2
        let captured_blocks = read_new_block_payloads(&dag, &sink_node_id).await;
        assert_eq!(captured_blocks.len(), 2, "Should have 2 BlockAncestor events");

        // Check the first event
        let new_block_1: BlockAncestorPayload = sonic_rs::from_str(&captured_blocks[0]).expect("Failed to parse first BlockAncestor");
        assert_eq!(new_block_1.height, 1, "First block should be root (height=1)");

        // Check the second event
        let new_block_2: BlockAncestorPayload = sonic_rs::from_str(&captured_blocks[1]).expect("Failed to parse second BlockAncestor");
        assert_eq!(new_block_2.height, 2, "Second block should be child (height=2)");

        // 10. Shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_requeue_unconnected_node() {
        // 1. Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // 2. Create the DAG
        let mut dag = ActorDAG::new();

        // 3. Create your BlockAncestorActor node (root)
        let new_block_actor = NewBlockActor::create_actor();
        let new_block_actor_id = new_block_actor.id();

        // 4. Add it as root
        let new_block_sender = dag.set_root(new_block_actor);

        // 5. Create a sink node for BlockAncestor
        let sink_node_id = "BlockAncestorSinkUnconnectedTest".to_string();
        let sink_node = create_new_block_sink_node(&sink_node_id)();
        dag.add_node(sink_node);
        dag.link_parent(&new_block_actor_id, &sink_node_id);

        // 6. Wrap the DAG and spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all(shutdown_rx).await;
            }
        });

        // 7. Step 1: Send the root block (height=1)
        let root_block = BlockAncestorPayload {
            height: 1,
            state_hash: "3N8aRootHash".to_string(),
            previous_state_hash: "".to_string(),
            last_vrf_output: "".to_string(),
        };
        new_block_sender
            .send(Event {
                event_type: EventType::BlockAncestor,
                payload: sonic_rs::to_string(&root_block).unwrap(),
            })
            .await
            .expect("Failed to send root block");
        sleep(Duration::from_millis(100)).await;

        // Verify we got one BlockAncestor event
        let captured_blocks = read_new_block_payloads(&dag, &sink_node_id).await;
        assert_eq!(captured_blocks.len(), 1, "Root block should produce 1 BlockAncestor");

        // 8. Step 2: Send an unconnected block at height=3 referencing a non-existent parent
        let unconnected_block = BlockAncestorPayload {
            height: 3,
            state_hash: "3N8aUnconnectedHash".to_string(),
            previous_state_hash: "3N8aNonExistentParent".to_string(),
            last_vrf_output: "".to_string(),
        };
        new_block_sender
            .send(Event {
                event_type: EventType::BlockAncestor,
                payload: sonic_rs::to_string(&unconnected_block).unwrap(),
            })
            .await
            .expect("Failed to send unconnected block");
        sleep(Duration::from_millis(100)).await;

        // 9. Verify that no new block event was added (still only 1 in total)
        let captured_blocks = read_new_block_payloads(&dag, &sink_node_id).await;
        assert_eq!(captured_blocks.len(), 1, "Expected no BlockAncestor event for the unconnected block");

        // 10. Shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }
}
