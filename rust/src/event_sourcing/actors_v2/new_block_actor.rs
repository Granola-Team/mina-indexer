use crate::{
    blockchain_tree::{BlockchainTree, Hash, Height, Node},
    event_sourcing::{
        actor_dag::{ActorNode, ActorNodeBuilder, ActorStore},
        events::{Event, EventType},
        managed_store::ManagedStore,
        payloads::BlockAncestorPayload,
    },
};
use log::warn;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct NewBlockActor;

const BLOCKCHAIN_TREE_KEY: &str = "blockchain_tree";
const BLOCKCHAIN_TREE_STORE: &str = "blockchain_tree_store";

impl NewBlockActor {
    pub async fn create_actor(preserve_data: bool) -> ActorNode {
        let mut state = ActorStore::new();

        let (blockchain_tree, managed_store) = BlockchainTree::load(preserve_data).await;
        state.insert(BLOCKCHAIN_TREE_KEY, blockchain_tree);
        state.insert(BLOCKCHAIN_TREE_STORE, managed_store);

        ActorNodeBuilder::new()
            .with_state(state)
            .with_processor(|event, state: Arc<Mutex<ActorStore>>, requeue| {
                Box::pin(async move {
                    if event.event_type == EventType::BlockAncestor {
                        let mut state = state.lock().await;
                        let mut blockchain_tree: BlockchainTree = state.remove(BLOCKCHAIN_TREE_KEY).unwrap();
                        let managed_store: ManagedStore = state.remove(BLOCKCHAIN_TREE_STORE).unwrap();

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
                            warn!("Unable to add block {:?}-{:?}", next_node.height, next_node.state_hash);
                            if let Ok((height, node)) = blockchain_tree.get_best_tip() {
                                warn!("Best tip is currently {:?}-{:?}", height, node.state_hash);
                            }

                            if let Err(err) = requeue.send(event).await {
                                warn!("Unable to requeue event: {err}");
                            }
                            state.insert(BLOCKCHAIN_TREE_KEY, blockchain_tree);
                            state.insert(BLOCKCHAIN_TREE_STORE, managed_store);
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

                        BlockchainTree::persist(&managed_store, &blockchain_tree).await;

                        state.insert(BLOCKCHAIN_TREE_KEY, blockchain_tree);
                        state.insert(BLOCKCHAIN_TREE_STORE, managed_store);

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
mod new_block_actor_tests_v2 {
    use super::NewBlockActor;
    use crate::event_sourcing::{
        actor_dag::{ActorDAG, ActorNode, ActorNodeBuilder, ActorStore},
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
    fn create_new_block_sink_node() -> ActorNode {
        ActorNodeBuilder::new()
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
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2. Create the DAG
        let mut dag = ActorDAG::new();

        // 3. Create your BlockAncestorActor node (root)
        let new_block_actor = NewBlockActor::create_actor(false).await;
        let new_block_actor_id = new_block_actor.id();

        // 4. Add it as root, which returns a `Sender<Event>`
        let new_block_sender = dag.set_root(new_block_actor);

        // 5. Create a sink node to capture `BlockAncestor` events
        let sink_node = create_new_block_sink_node();
        let sink_node_id = &sink_node.id();

        // 6. Add the sink node and link it to the actor
        dag.add_node(sink_node);
        dag.link_parent(&new_block_actor_id, sink_node_id);

        // 7. Wrap the DAG in an `Arc<Mutex<>>`
        let dag = Arc::new(Mutex::new(dag));

        // 8. Spawn the DAG
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
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
        let captured_blocks = read_new_block_payloads(&dag, sink_node_id).await;
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
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2. Create the DAG
        let mut dag = ActorDAG::new();

        // 3. Create your BlockAncestorActor node (root)
        let new_block_actor = NewBlockActor::create_actor(false).await;
        let new_block_actor_id = new_block_actor.id();

        // 4. Add it as root to get a `Sender<Event>`
        let new_block_sender = dag.set_root(new_block_actor);

        // 5. Create a sink node
        let sink_node = create_new_block_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&new_block_actor_id, sink_node_id);

        // 6. Wrap and spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
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
        let captured_blocks = read_new_block_payloads(&dag, sink_node_id).await;
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
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2. Create the DAG
        let mut dag = ActorDAG::new();

        // 3. Create your BlockAncestorActor node (root)
        let new_block_actor = NewBlockActor::create_actor(false).await;
        let new_block_actor_id = new_block_actor.id();

        // 4. Add it as root
        let new_block_sender = dag.set_root(new_block_actor);

        // 5. Create a sink node for BlockAncestor
        let sink_node = create_new_block_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&new_block_actor_id, sink_node_id);

        // 6. Wrap the DAG and spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
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
        let captured_blocks = read_new_block_payloads(&dag, sink_node_id).await;
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
        let captured_blocks = read_new_block_payloads(&dag, sink_node_id).await;
        assert_eq!(captured_blocks.len(), 1, "Expected no BlockAncestor event for the unconnected block");

        // 10. Shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_load_tree_and_reload() {
        // ------------------------------------------------
        // 1) Helper: Create a sink node to capture `NewBlock` events
        // ------------------------------------------------
        fn create_new_block_sink_node() -> ActorNode {
            ActorNodeBuilder::new()
                .with_state(ActorStore::new())
                .with_processor(|evt, st, _requeue| {
                    Box::pin(async move {
                        if evt.event_type == EventType::NewBlock {
                            let mut locked_store = st.lock().await;
                            let mut captured: Vec<String> = locked_store.get("captured_blocks").cloned().unwrap_or_default();
                            captured.push(evt.payload.clone());
                            locked_store.insert("captured_blocks", captured);
                        }
                        None
                    })
                })
                .build()
        }

        async fn read_new_block_payloads(dag: &Arc<Mutex<ActorDAG>>, sink_node_id: &str) -> Vec<String> {
            let dag_locked = dag.lock().await;
            let node_locked = dag_locked.read_node(sink_node_id.to_string()).expect("Sink node not found").lock().await;
            let store = node_locked.get_state();
            let locked_store = store.lock().await;
            locked_store.get::<Vec<String>>("captured_blocks").cloned().unwrap_or_default()
        }

        // ------------------------------------------------
        // 2) Build + spawn DAG to load and then store some blocks
        // ------------------------------------------------
        let (shutdown_tx1, _shutdown_rx1) = watch::channel(false);
        let mut dag_1 = ActorDAG::new();

        // a) First "run" => create the actor with `preserve_data = false` or `true` depending on your scenario
        //    This actor will create an empty BlockchainTree if not found in DB, then store updates on each block
        let actor_1 = NewBlockActor::create_actor(false).await;
        let actor_id_1 = actor_1.id();
        let actor_sender_1 = dag_1.set_root(actor_1);

        // b) Create sink node + link
        let sink_node_1 = create_new_block_sink_node();
        let sink_node_id_1 = sink_node_1.id();
        dag_1.add_node(sink_node_1);
        dag_1.link_parent(&actor_id_1, &sink_node_id_1);

        // c) Wrap + spawn
        let dag_1 = Arc::new(Mutex::new(dag_1));
        tokio::spawn({
            let dag = Arc::clone(&dag_1);
            async move { dag.lock().await.spawn_all().await }
        });

        // d) Insert a root block => height=10, state_hash="hash_root"
        let root_payload = BlockAncestorPayload {
            height: 10,
            state_hash: "hash_root".to_string(),
            previous_state_hash: "".to_string(),
            last_vrf_output: "".to_string(),
        };
        actor_sender_1
            .send(Event {
                event_type: EventType::BlockAncestor,
                payload: sonic_rs::to_string(&root_payload).unwrap(),
            })
            .await
            .expect("Failed to send root block #1");

        // e) Insert a child => height=11, referencing "hash_root"
        let child_payload = BlockAncestorPayload {
            height: 11,
            state_hash: "hash_child".to_string(),
            previous_state_hash: "hash_root".to_string(),
            last_vrf_output: "".to_string(),
        };
        actor_sender_1
            .send(Event {
                event_type: EventType::BlockAncestor,
                payload: sonic_rs::to_string(&child_payload).unwrap(),
            })
            .await
            .expect("Failed to send child block #1");

        // Wait for them to be processed
        sleep(Duration::from_millis(200)).await;

        // f) Check we got 2 `NewBlock` events so far
        let initial_blocks = read_new_block_payloads(&dag_1, &sink_node_id_1).await;
        assert_eq!(initial_blocks.len(), 2, "Should have 2 new blocks from first run");

        // g) "Shut down" by dropping the DAG + nodes.
        //    We can signal it, or just let `shutdown_tx1` be dropped:
        shutdown_tx1.send(true).expect("Failed to send shutdown #1");

        // Wait a bit so everything flushes out
        sleep(Duration::from_millis(100)).await;
        drop(dag_1); // Drop => triggers actor tasks to finish eventually

        // ------------------------------------------------
        // 3) Re-start the DAG => re-load the existing `BlockchainTree`
        // ------------------------------------------------
        let (shutdown_tx2, _shutdown_rx2) = watch::channel(false);
        let mut dag_2 = ActorDAG::new();

        // a) Create same actor => but now the DB table has a serialized tree with root=height=10 + child=height=11
        //    The actor should read that from the DB in `create_actor`, storing it in the ActorStore state
        let actor_2 = NewBlockActor::create_actor(false).await;
        let actor_id_2 = actor_2.id();
        let actor_sender_2 = dag_2.set_root(actor_2);

        // b) new sink node
        let sink_node_2 = create_new_block_sink_node();
        let sink_node_id_2 = sink_node_2.id();
        dag_2.add_node(sink_node_2);
        dag_2.link_parent(&actor_id_2, &sink_node_id_2);

        let dag_2 = Arc::new(Mutex::new(dag_2));
        tokio::spawn({
            let dag = Arc::clone(&dag_2);
            async move { dag.lock().await.spawn_all().await }
        });

        // c) Now let's add a new block => height=12, referencing "hash_child"
        let next_payload = BlockAncestorPayload {
            height: 12,
            state_hash: "hash_third".to_string(),
            previous_state_hash: "hash_child".to_string(),
            last_vrf_output: "".to_string(),
        };
        actor_sender_2
            .send(Event {
                event_type: EventType::BlockAncestor,
                payload: sonic_rs::to_string(&next_payload).unwrap(),
            })
            .await
            .expect("Failed to send block referencing loaded chain #2");

        // Wait
        sleep(Duration::from_millis(200)).await;

        // d) Check => we expect just 1 new block event in this second run, because the root+child were already loaded
        let second_run_blocks = read_new_block_payloads(&dag_2, &sink_node_id_2).await;
        assert_eq!(
            second_run_blocks.len(),
            1,
            "Should only see the newly added block (height=12), because root+child were loaded from DB, not re-broadcast"
        );

        // e) Optionally parse and confirm height=12
        let parsed_third: BlockAncestorPayload = sonic_rs::from_str(&second_run_blocks[0]).expect("Failed to parse the block at height=12");
        assert_eq!(
            parsed_third.previous_state_hash, "hash_child",
            "Should reference the existing child block as parent"
        );

        // f) Shutdown #2
        shutdown_tx2.send(true).expect("Failed to send shutdown #2");
        sleep(Duration::from_millis(50)).await;
    }
}
