use crate::{
    constants::TRANSITION_FRONTIER_DISTANCE,
    event_sourcing::{
        actor_dag::{ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
        canonical_items_manager::CanonicalItemsManager,
        events::{Event, EventType},
        payloads::{BlockCanonicityUpdatePayload, CanonicalMainnetBlockPayload, MainnetBlockPayload},
    },
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct CanonicalMainnetBlockActor;

const CANONICAL_MANAGER_KEY: &str = "canonical_manager";

#[async_trait]
impl ActorFactory for CanonicalMainnetBlockActor {
    async fn create_actor() -> ActorNode {
        let mut actor_store = ActorStore::new();
        actor_store.insert(
            CANONICAL_MANAGER_KEY,
            CanonicalItemsManager::<CanonicalMainnetBlockPayload>::new(TRANSITION_FRONTIER_DISTANCE as u64),
        );

        ActorNodeBuilder::new()
            .with_state(actor_store)
            .with_processor(|event, state: Arc<Mutex<ActorStore>>, _requeue| {
                Box::pin(async move {
                    let mut state = state.lock().await;
                    let manager: CanonicalItemsManager<CanonicalMainnetBlockPayload> = state.remove(CANONICAL_MANAGER_KEY).unwrap();

                    let output_events = match event.event_type {
                        EventType::BlockCanonicityUpdate => {
                            let canonicity_update: BlockCanonicityUpdatePayload = sonic_rs::from_str(&event.payload).unwrap();

                            manager.add_block_canonicity_update(canonicity_update.clone()).await;

                            // Process updates to publish canonical items
                            manager
                                .get_updates(canonicity_update.height)
                                .await
                                .into_iter()
                                .map(|item| Event {
                                    event_type: EventType::CanonicalMainnetBlock,
                                    payload: sonic_rs::to_string(&item).unwrap(),
                                })
                                .collect()
                        }
                        EventType::MainnetBlock => {
                            let mainnet_block: MainnetBlockPayload = sonic_rs::from_str(&event.payload).unwrap();

                            manager.add_items_count(mainnet_block.height, &mainnet_block.state_hash, 1).await;

                            let canonical_block = CanonicalMainnetBlockPayload {
                                canonical: false, // Default placeholder, updated later
                                was_canonical: false,
                                block: MainnetBlockPayload {
                                    height: mainnet_block.height,
                                    state_hash: mainnet_block.state_hash.clone(),
                                    previous_state_hash: mainnet_block.previous_state_hash.clone(),
                                    last_vrf_output: mainnet_block.last_vrf_output.clone(),
                                    user_command_count: mainnet_block.user_commands.len(),
                                    internal_command_count: mainnet_block.internal_command_count,
                                    user_commands: mainnet_block.user_commands.clone(),
                                    snark_work_count: mainnet_block.snark_work_count,
                                    snark_work: mainnet_block.snark_work.clone(),
                                    timestamp: mainnet_block.timestamp,
                                    coinbase_receiver: mainnet_block.coinbase_receiver.clone(),
                                    coinbase_reward_nanomina: mainnet_block.coinbase_reward_nanomina,
                                    global_slot_since_genesis: mainnet_block.global_slot_since_genesis,
                                    fee_transfer_via_coinbase: mainnet_block.fee_transfer_via_coinbase.clone(),
                                    fee_transfers: mainnet_block.fee_transfers.clone(),
                                    global_slot: mainnet_block.global_slot,
                                },
                            };

                            manager.add_item(canonical_block).await;

                            // Process updates to publish canonical items
                            manager
                                .get_updates(mainnet_block.height)
                                .await
                                .into_iter()
                                .map(|item| Event {
                                    event_type: EventType::CanonicalMainnetBlock,
                                    payload: sonic_rs::to_string(&item).unwrap(),
                                })
                                .collect()
                        }
                        _ => vec![],
                    };

                    if let Err(err) = manager.prune().await {
                        log::error!("Prune error: {}", err);
                    }

                    state.insert(CANONICAL_MANAGER_KEY, manager);

                    if output_events.is_empty() {
                        None
                    } else {
                        Some(output_events)
                    }
                })
            })
            .build()
    }
}

#[cfg(test)]
mod canonical_block_actor_tests_v2 {
    use super::CanonicalMainnetBlockActor;
    use crate::{
        constants::GENESIS_STATE_HASH,
        event_sourcing::{
            actor_dag::{ActorDAG, ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
            events::{Event, EventType},
            payloads::{BlockCanonicityUpdatePayload, CanonicalMainnetBlockPayload, MainnetBlockPayload},
        },
    };
    use sonic_rs;
    use std::sync::Arc;
    use tokio::{
        sync::{watch, Mutex},
        time::{sleep, Duration, Instant},
    };

    // ----------------------------------------------------------------
    // SINK NODE + HELPER FUNCTIONS
    // ----------------------------------------------------------------

    /// Creates a sink node that captures `CanonicalMainnetBlock` events in its state.
    fn create_canonical_block_sink_node() -> ActorNode {
        ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|event, state, _requeue| {
                Box::pin(async move {
                    // Only capture CanonicalMainnetBlock events
                    if event.event_type == EventType::CanonicalMainnetBlock {
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

    /// Reads all captured `CanonicalMainnetBlock` payloads from the sink node.
    async fn read_canonical_blocks(dag: &Arc<Mutex<ActorDAG>>, sink_node_id: &str) -> Vec<String> {
        let dag_locked = dag.lock().await;
        let sink_node_locked = dag_locked.read_node(sink_node_id.to_string()).expect("Sink node not found").lock().await;
        let store = sink_node_locked.get_state();
        let store_locked = store.lock().await;

        store_locked.get::<Vec<String>>("captured_blocks").cloned().unwrap_or_default()
    }

    /// Polls the sink node for `CanonicalMainnetBlock` events until one arrives or `deadline` elapses.
    /// Returns `Some(CanonicalMainnetBlockPayload)` if found, otherwise `None`.
    async fn poll_for_canonical_block(dag: &Arc<Mutex<ActorDAG>>, sink_node_id: &str, deadline: Duration) -> Option<CanonicalMainnetBlockPayload> {
        let start = Instant::now();
        while start.elapsed() < deadline {
            let updates = read_canonical_blocks(dag, sink_node_id).await;
            if !updates.is_empty() {
                // For simplicity, let’s just parse the first captured update
                return Some(sonic_rs::from_str(&updates[0]).expect("Failed to parse CanonicalMainnetBlockPayload"));
            }
            sleep(Duration::from_millis(50)).await;
        }
        None
    }

    // ----------------------------------------------------------------
    // TESTS
    // ----------------------------------------------------------------

    #[tokio::test]
    async fn test_block_canonicity_update_first() {
        // 1. Create the shutdown signal
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2. Create an ActorDAG
        let mut dag = ActorDAG::new();

        // 3. Create the CanonicalMainnetBlockActor node (root)
        let actor_node = CanonicalMainnetBlockActor::create_actor().await;
        let actor_id = actor_node.id();

        // 4. Set as root, obtaining a `Sender<Event>` for sending events
        let actor_sender = dag.set_root(actor_node);

        // 5. Create a sink node to capture `CanonicalMainnetBlock` events
        let sink_node = create_canonical_block_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, sink_node_id);

        // 6. Wrap the DAG in Arc<Mutex<>> and spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 7. Prepare data
        let block_payload = MainnetBlockPayload {
            height: 100,
            state_hash: "state_hash_100".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "vrf_output".to_string(),
            user_command_count: 0,
            internal_command_count: 0,
            user_commands: vec![],
            snark_work_count: 0,
            snark_work: vec![],
            timestamp: 0,
            coinbase_receiver: "receiver".to_string(),
            coinbase_reward_nanomina: 1000,
            global_slot_since_genesis: 10,
            fee_transfer_via_coinbase: None,
            fee_transfers: vec![],
            global_slot: 0,
        };

        let canonicity_update = BlockCanonicityUpdatePayload {
            height: 100,
            state_hash: "state_hash_100".to_string(),
            canonical: true,
            was_canonical: false,
        };

        // 8. Send BlockCanonicityUpdate first
        actor_sender
            .send(Event {
                event_type: EventType::BlockCanonicityUpdate,
                payload: sonic_rs::to_string(&canonicity_update).unwrap(),
            })
            .await
            .expect("Failed to send BlockCanonicityUpdate event");
        sleep(Duration::from_millis(100)).await;

        // Confirm no `CanonicalMainnetBlock` event is emitted yet
        let updates = read_canonical_blocks(&dag, sink_node_id).await;
        assert!(updates.is_empty(), "Unexpected CanonicalMainnetBlock event emitted before MainnetBlock");

        // 9. Send the MainnetBlock
        actor_sender
            .send(Event {
                event_type: EventType::MainnetBlock,
                payload: sonic_rs::to_string(&block_payload).unwrap(),
            })
            .await
            .expect("Failed to send MainnetBlock event");

        // 10. Wait (up to 2 seconds) for the `CanonicalMainnetBlock` event
        let update = poll_for_canonical_block(&dag, sink_node_id, Duration::from_secs(2)).await;
        if let Some(payload) = update {
            assert_eq!(payload.block.height, block_payload.height);
            assert_eq!(payload.block.state_hash, block_payload.state_hash);
            assert!(payload.canonical);
            assert!(!payload.was_canonical);
        } else {
            panic!("Expected CanonicalMainnetBlock event not received within 2s timeout.");
        }

        // 11. Shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_mainnet_block_first() {
        // 1. Create the shutdown signal
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2. Create an ActorDAG
        let mut dag = ActorDAG::new();

        // 3. Create the CanonicalMainnetBlockActor node (root)
        let actor_node = CanonicalMainnetBlockActor::create_actor().await;
        let actor_id = actor_node.id();
        let actor_sender = dag.set_root(actor_node);

        // 4. Create a sink node to capture `CanonicalMainnetBlock` events
        let sink_node = create_canonical_block_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, sink_node_id);

        // 5. Spawn the DAG
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 6. Prepare data
        let block_payload = MainnetBlockPayload {
            height: 100,
            state_hash: "state_hash_100".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "vrf_output".to_string(),
            user_command_count: 0,
            internal_command_count: 0,
            user_commands: vec![],
            snark_work_count: 0,
            snark_work: vec![],
            timestamp: 0,
            coinbase_receiver: "receiver".to_string(),
            coinbase_reward_nanomina: 1000,
            global_slot_since_genesis: 10,
            fee_transfer_via_coinbase: None,
            fee_transfers: vec![],
            global_slot: 0,
        };

        let canonicity_update = BlockCanonicityUpdatePayload {
            height: 100,
            state_hash: "state_hash_100".to_string(),
            canonical: true,
            was_canonical: false,
        };

        // 7. Send MainnetBlock first
        actor_sender
            .send(Event {
                event_type: EventType::MainnetBlock,
                payload: sonic_rs::to_string(&block_payload).unwrap(),
            })
            .await
            .expect("Failed to send MainnetBlock event");
        sleep(Duration::from_millis(100)).await;

        // Confirm no event is emitted yet (no canonicity info)
        let updates = read_canonical_blocks(&dag, sink_node_id).await;
        assert!(
            updates.is_empty(),
            "Unexpected CanonicalMainnetBlock event emitted before BlockCanonicityUpdate"
        );

        // 8. Send the canonicity update
        actor_sender
            .send(Event {
                event_type: EventType::BlockCanonicityUpdate,
                payload: sonic_rs::to_string(&canonicity_update).unwrap(),
            })
            .await
            .expect("Failed to send BlockCanonicityUpdate event");
        sleep(Duration::from_millis(100)).await;

        // 9. Expect the `CanonicalMainnetBlock` event now
        let update = poll_for_canonical_block(&dag, sink_node_id, Duration::from_secs(2)).await;
        if let Some(payload) = update {
            assert_eq!(payload.block.height, block_payload.height);
            assert_eq!(payload.block.state_hash, block_payload.state_hash);
            assert!(payload.canonical);
            assert!(!payload.was_canonical);
        } else {
            panic!("Expected CanonicalMainnetBlock event not received within 2s timeout.");
        }

        // 10. Shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }
}
