use crate::event_sourcing::{
    actor_dag::{ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
    events::{Event, EventType},
    payloads::{BerkeleyBlockPayload, BlockAncestorPayload, MainnetBlockPayload},
};
use async_trait::async_trait;
use sonic_rs::from_str;

pub struct BlockAncestorActor;

#[async_trait]
impl ActorFactory for BlockAncestorActor {
    async fn create_actor() -> ActorNode {
        ActorNodeBuilder::new() // Node listens for BerkeleyBlock and MainnetBlock
            .with_state(ActorStore::new())
            .with_processor(|event, _state, _requeue| {
                Box::pin(async move {
                    match event.event_type {
                        EventType::BerkeleyBlock => {
                            // Deserialize BerkeleyBlock payload
                            let block_payload: BerkeleyBlockPayload = from_str(&event.payload).unwrap();
                            let block_ancestor_payload = BlockAncestorPayload {
                                height: block_payload.height,
                                state_hash: block_payload.state_hash.clone(),
                                previous_state_hash: block_payload.previous_state_hash.clone(),
                                last_vrf_output: block_payload.last_vrf_output,
                            };
                            // Publish the BlockAncestor event
                            Some(vec![Event {
                                event_type: EventType::BlockAncestor,
                                payload: sonic_rs::to_string(&block_ancestor_payload).unwrap(),
                            }])
                        }
                        EventType::MainnetBlock => {
                            // Deserialize MainnetBlock payload
                            let block_payload: MainnetBlockPayload = from_str(&event.payload).unwrap();
                            let block_ancestor_payload = BlockAncestorPayload {
                                height: block_payload.height,
                                state_hash: block_payload.state_hash.clone(),
                                previous_state_hash: block_payload.previous_state_hash.clone(),
                                last_vrf_output: block_payload.last_vrf_output,
                            };

                            // Publish the BlockAncestor event
                            Some(vec![Event {
                                event_type: EventType::BlockAncestor,
                                payload: sonic_rs::to_string(&block_ancestor_payload).unwrap(),
                            }])
                        }
                        _ => None,
                    }
                })
            })
            .build()
    }
}

#[cfg(test)]
mod block_ancestor_actor_tests_v2 {
    use super::BlockAncestorActor;
    use crate::event_sourcing::{
        actor_dag::{ActorDAG, ActorFactory, ActorNodeBuilder, ActorStore},
        events::{Event, EventType},
        payloads::{BerkeleyBlockPayload, BlockAncestorPayload, MainnetBlockPayload},
    };
    use sonic_rs;
    use std::sync::Arc;
    use tokio::{
        sync::{watch, Mutex},
        time::{sleep, Duration},
    };

    #[tokio::test]
    async fn test_block_ancestor_actor_with_berkeley_block() {
        // 1. Create the shutdown signal
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2. Create an ActorDAG
        let mut dag = ActorDAG::new();

        // 3. Create your BlockAncestorActor node (root) using the ActorFactory
        let block_ancestor_node = BlockAncestorActor::create_actor().await;
        let block_ancestor_node_id = block_ancestor_node.id();

        // 4. Set the root in the DAG. This returns a Sender<Event> for sending events.
        let block_ancestor_sender = dag.set_root(block_ancestor_node);

        // 5. Create a sink node to capture `BlockAncestor` events
        let sink_node = ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|event, state, _requeue| {
                Box::pin(async move {
                    if event.event_type == EventType::BlockAncestor {
                        let mut locked_state = state.lock().await;
                        // Store all emitted BlockAncestor payloads in a vector
                        let mut captured_ancestors: Vec<String> = locked_state.get("captured_ancestors").cloned().unwrap_or_default();
                        captured_ancestors.push(event.payload.clone());
                        locked_state.insert("captured_ancestors", captured_ancestors);
                    }
                    None
                })
            })
            .build();
        let sink_node_id = sink_node.id();

        // 6. Add the sink node to the DAG and link it to the BlockAncestorActor
        dag.add_node(sink_node);
        dag.link_parent(&block_ancestor_node_id, &sink_node_id);

        // 7. Wrap the DAG in Arc<Mutex<>> so we can spawn it
        let dag = Arc::new(Mutex::new(dag));

        // 8. Spawn the DAG in the background
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 9. Create a sample BerkeleyBlockPayload
        let berkeley_block_payload = BerkeleyBlockPayload {
            height: 89,
            state_hash: "3NKVkEwELHY9CmPYxf25pwsKZpPf161QVCiC3JwdsyQwCYyE3wNCrRjWON".to_string(),
            previous_state_hash: "3NKJarZEsMAHkcPfhGA72eyjWBXGHergBZEoTuGXWS7vWeq8D5wu".to_string(),
            last_vrf_output: "hu0nffAHwdL0CYQNAlabyiUlwNWhlbj0MwynpKLtAAA=".to_string(),
            // Other fields can use default or be omitted
            ..Default::default()
        };

        // 10. Send a BerkeleyBlock event to the actor
        let event = Event {
            event_type: EventType::BerkeleyBlock,
            payload: sonic_rs::to_string(&berkeley_block_payload).unwrap(),
        };
        block_ancestor_sender.send(event).await.expect("Failed to send BerkeleyBlock event");

        // 11. Allow the DAG time to process
        sleep(Duration::from_millis(200)).await;

        // 12. Read the sink node's state to retrieve captured BlockAncestor payloads
        let sink_state = {
            let dag_locked = dag.lock().await;
            let sink_node_locked = dag_locked.read_node(sink_node_id.clone()).expect("Sink node not found").lock().await;
            let state = sink_node_locked.get_state();
            let store_locked = state.lock().await;
            store_locked.get::<Vec<String>>("captured_ancestors").cloned().unwrap_or_default()
        };

        // Expect exactly 1 payload so far
        assert_eq!(sink_state.len(), 1, "Should have 1 BlockAncestor payload");

        // 13. Deserialize the first (and only) captured payload
        let captured_payload: BlockAncestorPayload = sonic_rs::from_str(&sink_state[0]).expect("Failed to parse BlockAncestorPayload");

        assert_eq!(captured_payload.height, 89);
        assert_eq!(captured_payload.state_hash, "3NKVkEwELHY9CmPYxf25pwsKZpPf161QVCiC3JwdsyQwCYyE3wNCrRjWON");
        assert_eq!(captured_payload.previous_state_hash, "3NKJarZEsMAHkcPfhGA72eyjWBXGHergBZEoTuGXWS7vWeq8D5wu");

        // 14. Signal shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_block_ancestor_actor_with_mainnet_block() {
        // 1. Create a shutdown signal
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2. Create an ActorDAG
        let mut dag = ActorDAG::new();

        // 3. Create your BlockAncestorActor node (root)
        let block_ancestor_node = BlockAncestorActor::create_actor().await;
        let block_ancestor_node_id = block_ancestor_node.id();

        // 4. Set the root in the DAG. This returns a Sender<Event> for sending events.
        let block_ancestor_sender = dag.set_root(block_ancestor_node);

        // 5. Create a sink node for capturing BlockAncestor events
        let sink_node = ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|event, state, _requeue| {
                Box::pin(async move {
                    if event.event_type == EventType::BlockAncestor {
                        let mut locked_state = state.lock().await;
                        let mut captured_ancestors: Vec<String> = locked_state.get("captured_ancestors").cloned().unwrap_or_default();
                        captured_ancestors.push(event.payload.clone());
                        locked_state.insert("captured_ancestors", captured_ancestors);
                    }
                    None
                })
            })
            .build();
        let sink_node_id = sink_node.id();

        // 6. Add the sink node, link it to the actor
        dag.add_node(sink_node);
        dag.link_parent(&block_ancestor_node_id, &sink_node_id);

        // 7. Wrap the DAG in Arc<Mutex<>>
        let dag = Arc::new(Mutex::new(dag));

        // 8. Spawn the DAG
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 9. Create a sample MainnetBlockPayload
        let mainnet_block_payload = MainnetBlockPayload {
            height: 101,
            state_hash: "4MTNpwef32H67dHk9Mx25ZLpHfVz27QXECm8C4o5eyRa5LgJ1qLScCwpJM".to_string(),
            previous_state_hash: "4MPXcYhJY8URpwZxBEmv9C7kXf5h41PLXeX9GoTwFg3TuL2Q9zMn".to_string(),
            last_vrf_output: "WXPOLoGn9vE7HwqkE-K5bH4d3LmSPPJQcfoLsrTDkQA=".to_string(),
            // Other fields can use defaults
            ..Default::default()
        };

        // 10. Send a MainnetBlock event to the actor
        let event = Event {
            event_type: EventType::MainnetBlock,
            payload: sonic_rs::to_string(&mainnet_block_payload).unwrap(),
        };
        block_ancestor_sender.send(event).await.expect("Failed to send MainnetBlock event");

        // 11. Wait for processing
        sleep(Duration::from_millis(200)).await;

        // 12. Read sink node's state
        let sink_state = {
            let dag_locked = dag.lock().await;
            let sink_node_locked = dag_locked.read_node(sink_node_id.clone()).expect("Sink node not found").lock().await;
            let state = sink_node_locked.get_state();
            let store_locked = state.lock().await;
            store_locked.get::<Vec<String>>("captured_ancestors").cloned().unwrap_or_default()
        };

        // Should have exactly 1 captured payload
        assert_eq!(sink_state.len(), 1, "Should have 1 captured BlockAncestor event");

        // 13. Deserialize and verify
        let captured_payload: BlockAncestorPayload = sonic_rs::from_str(&sink_state[0]).expect("Failed to parse BlockAncestorPayload");

        assert_eq!(captured_payload.height, 101);
        assert_eq!(captured_payload.state_hash, "4MTNpwef32H67dHk9Mx25ZLpHfVz27QXECm8C4o5eyRa5LgJ1qLScCwpJM");
        assert_eq!(captured_payload.previous_state_hash, "4MPXcYhJY8URpwZxBEmv9C7kXf5h41PLXeX9GoTwFg3TuL2Q9zMn");

        // 14. Shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }
}
