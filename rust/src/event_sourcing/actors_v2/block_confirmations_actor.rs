use crate::{
    blockchain_tree::{BlockchainTree, Hash, Height, Node},
    event_sourcing::{
        actor_dag::{ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
        events::{Event, EventType},
        payloads::{BlockConfirmationPayload, NewBlockPayload},
    },
};
use log::warn;
use std::sync::Arc;
use tokio::sync::Mutex;

/// The key we use to store the `BlockchainTree` in the `ActorStore`
const BLOCKCHAIN_TREE_KEY: &str = "blockchain_tree";

/// Actor implementing the "block confirmation" logic
pub struct BlockConfirmationsActor;

impl BlockConfirmationsActor {
    /// Handle a `NewBlock` event.
    /// - Deserializes the `NewBlockPayload`
    /// - Retrieves the `BlockchainTree` from the store
    /// - Adds the new Node, then increments confirmations up its chain
    /// - Emits a `BlockConfirmation` once any parent node reaches 10 confirmations
    async fn on_new_block(event: Event, store: Arc<Mutex<ActorStore>>) -> Option<Vec<Event>> {
        let payload: NewBlockPayload = sonic_rs::from_str(&event.payload).expect("Failed to parse NewBlockPayload");

        // Lock store, retrieve the blockchain tree
        let mut store_locked = store.lock().await;
        let mut tree: BlockchainTree = store_locked.remove(BLOCKCHAIN_TREE_KEY).expect("Blockchain tree not found in store");

        let new_node = Node {
            height: Height(payload.height),
            state_hash: Hash(payload.state_hash.clone()),
            previous_state_hash: Hash(payload.previous_state_hash),
            last_vrf_output: payload.last_vrf_output,
            metadata_str: Some("0".to_string()),
            ..Default::default()
        };

        let mut out_events = Vec::new();

        if tree.is_empty() {
            // If tree is empty, set root
            if let Err(e) = tree.set_root(new_node.clone()) {
                warn!("Failed to set root: {e}");
            }
        } else if tree.has_parent(&new_node) {
            // Safe to add
            if let Err(e) = tree.add_node(new_node.clone()) {
                warn!("Failed to add node: {e}");
            } else {
                // Increment confirmations up the chain
                let mut iter_node = new_node;
                let mut confirmations = 0;
                while let Some(parent) = tree.get_parent_mut(&iter_node) {
                    parent.metadata_str = Some(confirmations.to_string());
                    confirmations += 1;

                    let confirm_event = Event {
                        event_type: EventType::BlockConfirmation,
                        payload: sonic_rs::to_string(&BlockConfirmationPayload {
                            height: parent.height.0,
                            state_hash: parent.state_hash.0.clone(),
                            confirmations,
                        })
                        .unwrap(),
                    };
                    out_events.push(confirm_event);

                    iter_node = parent.clone();
                }

                // Prune any older branches beyond your configured frontier
                if let Err(e) = tree.prune_tree() {
                    warn!("Prune tree error: {e}");
                }
            }
        } else {
            warn!("No parent found for block height={} state_hash={}", new_node.height.0, new_node.state_hash.0);
        }

        // Store the updated tree back
        store_locked.insert(BLOCKCHAIN_TREE_KEY, tree);

        if out_events.is_empty() {
            None
        } else {
            Some(out_events)
        }
    }
}

#[async_trait::async_trait]
impl ActorFactory for BlockConfirmationsActor {
    /// Build an `ActorNode` with a `BlockchainTree` in its `ActorStore`, capacity=11
    async fn create_actor() -> ActorNode {
        // 1) Create the store, insert a brand new `BlockchainTree`
        let mut store = ActorStore::new();
        // e.g. prune distance = 11
        store.insert(BLOCKCHAIN_TREE_KEY, BlockchainTree::new(11));

        // 2) Return the built node
        ActorNodeBuilder::new()
            .with_state(store)
            .with_processor(|event, store, _requeue| {
                Box::pin(async move {
                    match event.event_type {
                        EventType::NewBlock => Self::on_new_block(event, store).await,
                        _ => None,
                    }
                })
            })
            .build()
    }
}

#[cfg(test)]
mod block_confirmations_actor_tests_v2 {
    use super::BlockConfirmationsActor;
    use crate::{
        event_sourcing::{
            actor_dag::{ActorDAG, ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
            events::{Event, EventType},
            payloads::{BlockConfirmationPayload, NewBlockPayload},
        },
        // ...
    };
    use std::sync::Arc;
    use tokio::{
        sync::Mutex,
        time::{sleep, Duration},
    };

    /// Minimal sink node that captures `BlockConfirmation` events.
    /// We store each event’s payload (the JSON-serialized `BlockConfirmationPayload`) in a Vec<String>.
    fn create_confirmation_sink_node() -> ActorNode {
        ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|evt, store, _requeue| {
                Box::pin(async move {
                    if evt.event_type == EventType::BlockConfirmation {
                        let mut locked_store = store.lock().await;
                        let mut captured: Vec<String> = locked_store.get("captured_confirmations").cloned().unwrap_or_default();
                        captured.push(evt.payload.clone());
                        locked_store.insert("captured_confirmations", captured);
                    }
                    None
                })
            })
            .build()
    }

    /// Drains (removes) the "captured_confirmations" vector from the sink node's store
    /// and returns a `Vec<BlockConfirmationPayload>`.
    async fn drain_captured_confirmations(dag: &Arc<Mutex<ActorDAG>>, sink_node_id: &str) -> Vec<BlockConfirmationPayload> {
        let dag_locked = dag.lock().await;
        let sink_node_locked = dag_locked.read_node(sink_node_id.to_string()).expect("Sink node not found").lock().await;

        let sink_state = sink_node_locked.get_state();
        let mut store_locked = sink_state.lock().await;

        // Remove the entire Vec<String> so we "drain" it from the store
        let raw_events: Vec<String> = store_locked.remove("captured_confirmations").unwrap_or_default();

        // Now parse each JSON payload into a `BlockConfirmationPayload`
        raw_events
            .into_iter()
            .map(|json_str| sonic_rs::from_str::<BlockConfirmationPayload>(&json_str).expect("Failed to parse BlockConfirmationPayload from sink store"))
            .collect()
    }

    #[tokio::test]
    async fn test_root_block_has_no_confirmations() {
        // 1) Build an ActorDAG
        let mut dag = ActorDAG::new();

        // 2) Create the block confirmations actor
        let bc_actor = BlockConfirmationsActor::create_actor().await;
        let bc_actor_id = bc_actor.id();

        // 3) Add it as root => get a `Sender<Event>`
        let bc_sender = dag.set_root(bc_actor);

        // 4) Create + link the sink node
        let sink_node = create_confirmation_sink_node();
        let sink_id = sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&bc_actor_id, &sink_id);

        // 5) Wrap + spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move { dag.lock().await.spawn_all().await }
        });

        // 6) Send a “NewBlock” event representing the root block
        let root_block = NewBlockPayload {
            height: 0,
            state_hash: "hash_root".to_string(),
            previous_state_hash: "".to_string(),
            last_vrf_output: "vrf_root".to_string(),
        };
        bc_sender
            .send(Event {
                event_type: EventType::NewBlock,
                payload: sonic_rs::to_string(&root_block).unwrap(),
            })
            .await
            .expect("Failed to send root block event");

        // Wait a bit to let the DAG process
        sleep(Duration::from_millis(50)).await;

        // 7) Verify no confirmations were emitted
        let confirmations = drain_captured_confirmations(&dag, &sink_id).await;
        assert!(confirmations.is_empty(), "Root block should not produce confirmations");
    }

    #[tokio::test]
    async fn test_block_confirmations_for_deep_chain() {
        // 1) Build an ActorDAG
        let mut dag = ActorDAG::new();

        // 2) Create the block confirmations actor
        let bc_actor = BlockConfirmationsActor::create_actor().await;
        let bc_actor_id = bc_actor.id();
        let bc_sender = dag.set_root(bc_actor);

        // 3) Sink node
        let sink_node = create_confirmation_sink_node();
        let sink_id = sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&bc_actor_id, &sink_id);

        // 4) Wrap + spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move { dag.lock().await.spawn_all().await }
        });

        // 5) Create & send blocks 0..10
        for i in 0..=10 {
            let event = Event {
                event_type: EventType::NewBlock,
                payload: sonic_rs::to_string(&NewBlockPayload {
                    height: i,
                    state_hash: format!("hash_{}", i),
                    previous_state_hash: if i == 0 { "".to_string() } else { format!("hash_{}", i - 1) },
                    last_vrf_output: format!("vrf_{}", i),
                })
                .unwrap(),
            };
            bc_sender.send(event).await.unwrap();
        }

        // Wait for the chain to process
        sleep(Duration::from_millis(100)).await;

        // Clear out any confirmations that might have been generated
        let _ = drain_captured_confirmations(&dag, &sink_id).await;

        // 6) Now add block #11 => might cause a block up the chain to reach 10 confirmations
        let event_11 = Event {
            event_type: EventType::NewBlock,
            payload: sonic_rs::to_string(&NewBlockPayload {
                height: 11,
                state_hash: "hash_11".to_string(),
                previous_state_hash: "hash_10".to_string(),
                last_vrf_output: "vrf_11".to_string(),
            })
            .unwrap(),
        };
        bc_sender.send(event_11).await.unwrap();

        // Wait & read confirmations
        sleep(Duration::from_millis(100)).await;
        let confirmations = drain_captured_confirmations(&dag, &sink_id).await;

        // Suppose your logic increments the parent's confirmations each time.
        // If you expect *exactly* one new event from block #11:
        assert_eq!(confirmations.len(), 10, "Should produce exactly 10 block-confirmations event from block #11");

        {
            let c = &confirmations.last().unwrap();
            // Check that this matches your logic from the old test (for example)
            assert_eq!(c.height, 1, "Expected a confirmation on the block at height=1");
            assert_eq!(c.state_hash, "hash_1");
            assert_eq!(c.confirmations, 10, "Expected that block #1 now has 10 confirmations");
        }
        {
            let c = &confirmations[confirmations.len() - 2];
            // Check that this matches your logic from the old test (for example)
            assert_eq!(c.height, 2, "Expected a confirmation on the block at height=2");
            assert_eq!(c.state_hash, "hash_2");
            assert_eq!(c.confirmations, 9, "Expected that block #2 now has 9 confirmations");
        }
        // ... skip a few
        {
            let c = &confirmations[1];
            // Check that this matches your logic from the old test (for example)
            assert_eq!(c.height, 9, "Expected a confirmation on the block at height=9");
            assert_eq!(c.state_hash, "hash_9");
            assert_eq!(c.confirmations, 2, "Expected that block #9 now has 2 confirmations");
        }
        {
            let c = &confirmations[0];
            // Check that this matches your logic from the old test (for example)
            assert_eq!(c.height, 10, "Expected a confirmation on the block at height=10");
            assert_eq!(c.state_hash, "hash_10");
            assert_eq!(c.confirmations, 1, "Expected that block #10 now has 1 confirmations");
        }
    }
}
