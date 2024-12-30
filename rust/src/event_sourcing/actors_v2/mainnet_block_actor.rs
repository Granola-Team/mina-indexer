use crate::{
    event_sourcing::{
        actor_dag::{ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
        block::BlockTrait,
        events::{Event, EventType},
        mainnet_block_models::MainnetBlock,
        payloads::MainnetBlockPayload,
    },
    utility::{extract_height_and_hash, get_cleaned_pcb},
};
use async_trait::async_trait;
use std::path::Path;

pub struct MainnetBlockParserActor;

#[async_trait]
impl ActorFactory for MainnetBlockParserActor {
    async fn create_actor() -> ActorNode {
        ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|event, _state, _requeue| {
                Box::pin(async move {
                    match event.event_type {
                        EventType::MainnetBlockPath => {
                            // Parse the block path and contents
                            let (height, state_hash) = extract_height_and_hash(Path::new(&event.payload));
                            let file_content = get_cleaned_pcb(&event.payload).expect("File should exist and be readable");
                            let block: MainnetBlock = sonic_rs::from_str(&file_content).expect("Failed to parse block");

                            // Create block payload
                            let block_payload = MainnetBlockPayload {
                                height: height as u64,
                                global_slot: block.get_global_slot_since_genesis(),
                                state_hash: state_hash.to_string(),
                                previous_state_hash: block.get_previous_state_hash(),
                                last_vrf_output: block.get_last_vrf_output(),
                                user_command_count: block.get_user_commands_count(),
                                snark_work_count: block.get_aggregated_snark_work().len(),
                                snark_work: block.get_aggregated_snark_work(),
                                timestamp: block.get_timestamp(),
                                coinbase_reward_nanomina: block.get_coinbase_reward_nanomina(),
                                coinbase_receiver: block.get_coinbase_receiver(),
                                global_slot_since_genesis: block.get_global_slot_since_genesis(),
                                user_commands: block.get_user_commands(),
                                fee_transfer_via_coinbase: block.get_fee_transfers_via_coinbase(),
                                fee_transfers: block.get_fee_transfers(),
                                internal_command_count: block.get_internal_command_count(),
                            };

                            // Publish the MainnetBlock event
                            Some(vec![Event {
                                event_type: EventType::MainnetBlock,
                                payload: sonic_rs::to_string(&block_payload).unwrap(),
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
mod mainnet_block_actor_tests_v2 {
    use super::MainnetBlockParserActor;
    use crate::event_sourcing::{
        actor_dag::{ActorDAG, ActorFactory, ActorNodeBuilder, ActorStore},
        events::{Event, EventType},
        payloads::MainnetBlockPayload,
    };
    use sonic_rs;
    use std::sync::Arc;
    use tokio::{
        sync::{watch, Mutex},
        time::{sleep, Duration},
    };

    #[tokio::test]
    async fn test_mainnet_block_parser_actor_with_real_files() {
        // 1. Create a shutdown signal
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2. Create an ActorDAG
        let mut dag = ActorDAG::new();

        // 3. Create your parser actor node (root) using the ActorFactory
        let parser_node = MainnetBlockParserActor::create_actor().await;
        let parser_node_id = parser_node.id();

        // 4. Add the parser node as the root. This returns a Sender<Event> for sending events.
        let parser_sender = dag.set_root(parser_node);

        // 5. Create a "sink" node that captures `MainnetBlock` events by storing payloads in its ActorStore.
        let sink_node = ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|event, state, _requeue| {
                Box::pin(async move {
                    if event.event_type == EventType::MainnetBlock {
                        let mut locked_state = state.lock().await;
                        // Store all emitted MainnetBlock payloads in a vector
                        let mut captured_blocks: Vec<String> = locked_state.get("captured_blocks").cloned().unwrap_or_default();
                        captured_blocks.push(event.payload.clone());
                        locked_state.insert("captured_blocks", captured_blocks);
                    }
                    None
                })
            })
            .build();
        let sink_node_id = sink_node.id();

        // 6. Add the sink node and link it to the parser node
        dag.add_node(sink_node);
        dag.link_parent(&parser_node_id, &sink_node_id);

        // 7. Wrap the DAG in Arc<Mutex<>> so it can be spawned in the background
        let dag = Arc::new(Mutex::new(dag));

        // 8. Spawn the entire DAG
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 9. Paths for two test blocks
        let block_file_100 = "./src/event_sourcing/test_data/100_mainnet_blocks/mainnet-100-3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4.json";
        let block_file_99 = "./src/event_sourcing/test_data/100_mainnet_blocks/mainnet-99-3NLdywCHZmuqxS4hUnW7Uuu6sr97iifh5Ldc6m9EbzVZyLqbxqCh.json";

        // ===== Scenario 1: Block 100 =====
        let event_100 = Event {
            event_type: EventType::MainnetBlockPath,
            payload: block_file_100.to_string(),
        };
        // Send the file path to the parser actor
        parser_sender.send(event_100).await.expect("Failed to send event for block 100");

        // Give the DAG time to process
        sleep(Duration::from_millis(200)).await;

        // Read the sink node's state to retrieve the captured block payload
        let sink_state = {
            let dag_locked = dag.lock().await;
            let sink_node_locked = dag_locked.read_node(sink_node_id.clone()).unwrap().lock().await;
            let state = sink_node_locked.get_state();
            let store_locked = state.lock().await;
            store_locked.get::<Vec<String>>("captured_blocks").cloned().unwrap_or_default()
        };

        // We expect exactly 1 payload so far
        assert_eq!(sink_state.len(), 1, "Should have 1 block payload in the sink node");
        let payload_100 = &sink_state[0];
        let parsed_block_100: MainnetBlockPayload = sonic_rs::from_str(payload_100).unwrap();

        assert_eq!(parsed_block_100.height, 100);
        assert_eq!(parsed_block_100.state_hash, "3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4");
        assert_eq!(parsed_block_100.previous_state_hash, "3NLdywCHZmuqxS4hUnW7Uuu6sr97iifh5Ldc6m9EbzVZyLqbxqCh");
        assert_eq!(parsed_block_100.last_vrf_output, "HXzRY01h73mWXp4cjNwdDTYLDtdFU5mYhTbWWi-1wwE=");
        assert_eq!(parsed_block_100.user_command_count, 1);
        assert_eq!(parsed_block_100.snark_work_count, 0);

        // ===== Scenario 2: Block 99 =====
        let event_99 = Event {
            event_type: EventType::MainnetBlockPath,
            payload: block_file_99.to_string(),
        };
        // Send the file path to the parser actor
        parser_sender.send(event_99).await.expect("Failed to send event for block 99");

        // Give the DAG time to process
        sleep(Duration::from_millis(200)).await;

        // Read the sink node's state again
        let sink_state = {
            let dag_locked = dag.lock().await;
            let sink_node_locked = dag_locked.read_node(sink_node_id.clone()).unwrap().lock().await;
            let state = sink_node_locked.get_state();
            let store_locked = state.lock().await;
            store_locked.get::<Vec<String>>("captured_blocks").cloned().unwrap_or_default()
        };

        // We now expect 2 payloads total
        assert_eq!(sink_state.len(), 2, "Should have 2 block payloads in the sink node");
        let payload_99 = &sink_state[1];
        let parsed_block_99: MainnetBlockPayload = sonic_rs::from_str(payload_99).unwrap();

        assert_eq!(parsed_block_99.height, 99);
        assert_eq!(parsed_block_99.state_hash, "3NLdywCHZmuqxS4hUnW7Uuu6sr97iifh5Ldc6m9EbzVZyLqbxqCh");
        assert_eq!(parsed_block_99.previous_state_hash, "3NLAuBJPgT4Tk4LpufAEDQq4Jv9QVUefq3n3eB9x9VgGqe6LKzWp");
        assert_eq!(parsed_block_99.last_vrf_output, "ws1xspEgjEyLiSS0V2-Egf9UzJG3FACpruvvDEsqDAA=");
        assert_eq!(parsed_block_99.user_command_count, 3);
        assert_eq!(parsed_block_99.snark_work_count, 0);

        // ===== Shutdown =====
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }
}
