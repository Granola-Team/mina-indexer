use crate::{
    constants::TRANSITION_FRONTIER_DISTANCE,
    event_sourcing::{
        actor_dag::{ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
        canonical_items_manager::CanonicalItemsManager,
        events::{Event, EventType},
        payloads::{BerkeleyBlockPayload, BlockCanonicityUpdatePayload, CanonicalBerkeleyBlockPayload},
    },
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct CanonicalBerkeleyBlockActor;

const CANONICAL_MANAGER_KEY: &str = "canonical_manager";

#[async_trait]
impl ActorFactory for CanonicalBerkeleyBlockActor {
    async fn create_actor() -> ActorNode {
        let mut actor_store = ActorStore::new();
        actor_store.insert(
            CANONICAL_MANAGER_KEY,
            CanonicalItemsManager::<CanonicalBerkeleyBlockPayload>::new(TRANSITION_FRONTIER_DISTANCE as u64),
        );

        ActorNodeBuilder::new()
            .with_state(actor_store)
            .with_processor(|event, state: Arc<Mutex<ActorStore>>, _requeue| {
                Box::pin(async move {
                    let mut state = state.lock().await;
                    let manager: CanonicalItemsManager<CanonicalBerkeleyBlockPayload> = state.remove(CANONICAL_MANAGER_KEY).unwrap();

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
                                    event_type: EventType::CanonicalBerkeleyBlock,
                                    payload: sonic_rs::to_string(&item).unwrap(),
                                })
                                .collect()
                        }
                        EventType::BerkeleyBlock => {
                            let berkeley_block: BerkeleyBlockPayload = sonic_rs::from_str(&event.payload).unwrap();

                            manager.add_items_count(berkeley_block.height, &berkeley_block.state_hash, 1).await;

                            let canonical_block = CanonicalBerkeleyBlockPayload {
                                canonical: false, // Default placeholder, updated later
                                was_canonical: false,
                                block: BerkeleyBlockPayload {
                                    height: berkeley_block.height,
                                    state_hash: berkeley_block.state_hash.clone(),
                                    previous_state_hash: berkeley_block.previous_state_hash.clone(),
                                    last_vrf_output: berkeley_block.last_vrf_output.clone(),
                                    user_command_count: berkeley_block.user_commands.len(),
                                    zk_app_command_count: berkeley_block.zk_app_command_count,
                                    user_commands: berkeley_block.user_commands.clone(),
                                    zk_app_commands: berkeley_block.zk_app_commands.clone(),
                                    snark_work_count: berkeley_block.snark_work_count,
                                    snark_work: berkeley_block.snark_work.clone(),
                                    timestamp: berkeley_block.timestamp,
                                    coinbase_receiver: berkeley_block.coinbase_receiver.clone(),
                                    coinbase_reward_nanomina: berkeley_block.coinbase_reward_nanomina,
                                    global_slot_since_genesis: berkeley_block.global_slot_since_genesis,
                                    fee_transfer_via_coinbase: berkeley_block.fee_transfer_via_coinbase.clone(),
                                    fee_transfers: berkeley_block.fee_transfers.clone(),
                                },
                            };

                            manager.add_item(canonical_block).await;

                            // Process updates to publish canonical items
                            manager
                                .get_updates(berkeley_block.height)
                                .await
                                .into_iter()
                                .map(|item| Event {
                                    event_type: EventType::CanonicalBerkeleyBlock,
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
mod canonical_berkeley_block_actor_tests_v2 {
    use super::CanonicalBerkeleyBlockActor;
    use crate::{
        constants::GENESIS_STATE_HASH,
        event_sourcing::{
            actor_dag::{ActorDAG, ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
            events::{Event, EventType},
            payloads::{BerkeleyBlockPayload, BlockCanonicityUpdatePayload, CanonicalBerkeleyBlockPayload},
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

    /// Creates a sink node to capture `CanonicalBerkeleyBlock` events in its internal store.
    fn create_canonical_berkeley_sink_node() -> ActorNode {
        ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|event, state, _requeue| {
                Box::pin(async move {
                    if event.event_type == EventType::CanonicalBerkeleyBlock {
                        let mut store = state.lock().await;
                        let mut captured: Vec<String> = store.get("captured_blocks").cloned().unwrap_or_default();
                        captured.push(event.payload.clone());
                        store.insert("captured_blocks", captured);
                    }
                    None
                })
            })
            .build()
    }

    /// Reads all captured `CanonicalBerkeleyBlock` payloads (as JSON strings) from the sink node.
    async fn read_captured_blocks(dag: &Arc<Mutex<ActorDAG>>, sink_node_id: &str) -> Vec<String> {
        let dag_locked = dag.lock().await;
        let sink_node_locked = dag_locked.read_node(sink_node_id.to_string()).expect("Sink node not found").lock().await;

        let store = sink_node_locked.get_state();
        let store_locked = store.lock().await;
        store_locked.get::<Vec<String>>("captured_blocks").cloned().unwrap_or_default()
    }

    /// Polls the sink node up to `timeout` until it has at least one `CanonicalBerkeleyBlock`.
    /// Returns the first captured block if found, otherwise `None`.
    async fn poll_for_canonical_berkeley_block(dag: &Arc<Mutex<ActorDAG>>, sink_node_id: &str, timeout: Duration) -> Option<CanonicalBerkeleyBlockPayload> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            let blocks = read_captured_blocks(dag, sink_node_id).await;
            if !blocks.is_empty() {
                // parse the first block
                return Some(sonic_rs::from_str(&blocks[0]).expect("Failed to parse CanonicalBerkeleyBlockPayload"));
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
        // 1. Create a shutdown signal
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2. Build an ActorDAG
        let mut dag = ActorDAG::new();

        // 3. Create the root actor (CanonicalBerkeleyBlockActor) using ActorFactory
        let actor_node = CanonicalBerkeleyBlockActor::create_actor().await;
        let actor_id = actor_node.id();

        // 4. Set it as root; get a Sender<Event>
        let actor_sender = dag.set_root(actor_node);

        // 5. Create the sink node for capturing `CanonicalBerkeleyBlock` events
        let sink_node = create_canonical_berkeley_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, sink_node_id);

        // 6. Wrap the DAG in Arc<Mutex<>> so we can spawn it
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 7. Create test data
        let block_payload = BerkeleyBlockPayload {
            height: 100,
            state_hash: "state_hash_100".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "vrf_output".to_string(),
            user_command_count: 0,
            zk_app_command_count: 0,
            user_commands: vec![],
            zk_app_commands: vec![],
            snark_work_count: 0,
            snark_work: vec![],
            timestamp: 0,
            coinbase_receiver: "receiver".to_string(),
            coinbase_reward_nanomina: 1000,
            global_slot_since_genesis: 10,
            fee_transfer_via_coinbase: None,
            fee_transfers: vec![],
        };

        let canonicity_update = BlockCanonicityUpdatePayload {
            height: 100,
            state_hash: "state_hash_100".to_string(),
            canonical: true,
            was_canonical: false,
        };

        // 8. Send the canonicity update first
        actor_sender
            .send(Event {
                event_type: EventType::BlockCanonicityUpdate,
                payload: sonic_rs::to_string(&canonicity_update).unwrap(),
            })
            .await
            .expect("Failed to send BlockCanonicityUpdate event");
        sleep(Duration::from_millis(100)).await;

        // Confirm no canonical block is produced yet
        let blocks = read_captured_blocks(&dag, sink_node_id).await;
        assert!(blocks.is_empty(), "Unexpected CanonicalBerkeleyBlock event before BerkeleyBlock");

        // 9. Send the BerkeleyBlock
        actor_sender
            .send(Event {
                event_type: EventType::BerkeleyBlock,
                payload: sonic_rs::to_string(&block_payload).unwrap(),
            })
            .await
            .expect("Failed to send BerkeleyBlock event");

        // 10. Wait up to 2s for the canonical block to appear
        let canonical_block = poll_for_canonical_berkeley_block(&dag, sink_node_id, Duration::from_secs(2)).await;
        if let Some(cblock) = canonical_block {
            assert_eq!(cblock.block.height, block_payload.height);
            assert_eq!(cblock.block.state_hash, block_payload.state_hash);
            assert!(cblock.canonical);
            assert!(!cblock.was_canonical);
        } else {
            panic!("Expected CanonicalBerkeleyBlock event not received within 2s timeout.");
        }

        // 11. Shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_berkeley_block_first() {
        // 1. Shutdown signal
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2. Build an ActorDAG
        let mut dag = ActorDAG::new();

        // 3. Create the root actor
        let actor_node = CanonicalBerkeleyBlockActor::create_actor().await;
        let actor_id = actor_node.id();
        let actor_sender = dag.set_root(actor_node);

        // 4. Create the sink node for capturing CanonicalBerkeleyBlock
        let sink_node = create_canonical_berkeley_sink_node();
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

        // 6. Data
        let block_payload = BerkeleyBlockPayload {
            height: 100,
            state_hash: "state_hash_100".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "vrf_output".to_string(),
            user_command_count: 0,
            zk_app_command_count: 0,
            user_commands: vec![],
            zk_app_commands: vec![],
            snark_work_count: 0,
            snark_work: vec![],
            timestamp: 0,
            coinbase_receiver: "receiver".to_string(),
            coinbase_reward_nanomina: 1000,
            global_slot_since_genesis: 10,
            fee_transfer_via_coinbase: None,
            fee_transfers: vec![],
        };

        let canonicity_update = BlockCanonicityUpdatePayload {
            height: 100,
            state_hash: "state_hash_100".to_string(),
            canonical: true,
            was_canonical: false,
        };

        // 7. Send the BerkeleyBlock first
        actor_sender
            .send(Event {
                event_type: EventType::BerkeleyBlock,
                payload: sonic_rs::to_string(&block_payload).unwrap(),
            })
            .await
            .expect("Failed to send BerkeleyBlock event");
        sleep(Duration::from_millis(100)).await;

        // Confirm no canonical block is emitted yet
        let blocks = read_captured_blocks(&dag, sink_node_id).await;
        assert!(blocks.is_empty(), "Unexpected CanonicalBerkeleyBlock event before canonicity update");

        // 8. Then send the canonicity update
        actor_sender
            .send(Event {
                event_type: EventType::BlockCanonicityUpdate,
                payload: sonic_rs::to_string(&canonicity_update).unwrap(),
            })
            .await
            .expect("Failed to send BlockCanonicityUpdate event");

        // Wait for the canonical block to appear
        let canonical_block = poll_for_canonical_berkeley_block(&dag, sink_node_id, Duration::from_secs(2)).await;
        if let Some(cblock) = canonical_block {
            assert_eq!(cblock.block.height, block_payload.height);
            assert_eq!(cblock.block.state_hash, block_payload.state_hash);
            assert!(cblock.canonical);
            assert!(!cblock.was_canonical);
        } else {
            panic!("Expected CanonicalBerkeleyBlock event not received within 2s timeout.");
        }

        // 9. Shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }
}
