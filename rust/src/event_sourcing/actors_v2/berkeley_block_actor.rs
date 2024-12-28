use crate::{
    event_sourcing::{
        actor_dag::{ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
        berkeley_block_models::BerkeleyBlock,
        block::BlockTrait,
        events::{Event, EventType},
        payloads::BerkeleyBlockPayload,
    },
    utility::extract_height_and_hash,
};
use async_trait::async_trait;
use std::{fs, path::Path};

pub struct BerkeleyBlockActor;

#[async_trait]
impl ActorFactory for BerkeleyBlockActor {
    async fn create_actor() -> ActorNode {
        ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|event, _state, _requeue| {
                Box::pin(async move {
                    if event.event_type == EventType::BerkeleyBlockPath {
                        // Parse the block path and contents
                        let (height, state_hash) = extract_height_and_hash(Path::new(&event.payload));
                        let file_content = fs::read_to_string(Path::new(&event.payload)).expect("Failed to read JSON file from disk");
                        let berkeley_block: BerkeleyBlock = sonic_rs::from_str(&file_content).expect("Failed to parse Berkeley block");

                        // Create the Berkeley block payload
                        let berkeley_block_payload = BerkeleyBlockPayload {
                            height: height as u64,
                            state_hash: state_hash.to_string(),
                            previous_state_hash: berkeley_block.get_previous_state_hash(),
                            last_vrf_output: berkeley_block.get_last_vrf_output(),
                            user_command_count: berkeley_block.get_user_commands_count(),
                            user_commands: berkeley_block.get_user_commands(),
                            zk_app_command_count: berkeley_block.get_zk_app_commands_count(),
                            zk_app_commands: berkeley_block.get_zk_app_commands(),
                            snark_work_count: berkeley_block.get_aggregated_snark_work().len(),
                            snark_work: berkeley_block.get_aggregated_snark_work(),
                            fee_transfers: berkeley_block.get_fee_transfers(),
                            fee_transfer_via_coinbase: berkeley_block.get_fee_transfers_via_coinbase(),
                            timestamp: berkeley_block.get_timestamp(),
                            coinbase_receiver: berkeley_block.get_coinbase_receiver(),
                            coinbase_reward_nanomina: berkeley_block.get_coinbase_reward_nanomina(),
                            global_slot_since_genesis: berkeley_block.get_global_slot_since_genesis(),
                        };

                        // Publish the BerkeleyBlock event
                        Some(vec![Event {
                            event_type: EventType::BerkeleyBlock,
                            payload: sonic_rs::to_string(&berkeley_block_payload).unwrap(),
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
mod berkeley_block_actor_tests_v2 {
    use super::BerkeleyBlockActor;
    use crate::event_sourcing::{
        actor_dag::{ActorDAG, ActorFactory, ActorNodeBuilder, ActorStore},
        events::{Event, EventType},
        payloads::BerkeleyBlockPayload,
    };
    use sonic_rs;
    use std::sync::Arc;
    use tokio::{
        sync::{watch, Mutex},
        time::{sleep, Duration},
    };

    #[tokio::test]
    async fn test_berkeley_block_parser_actor_with_real_files() {
        // 1. Create a shutdown signal
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2. Create an ActorDAG
        let mut dag = ActorDAG::new();

        // 3. Create your BerkeleyBlockActor node (root) using the ActorFactory
        let parser_node = BerkeleyBlockActor::create_actor().await;
        let parser_node_id = parser_node.id();

        // 4. Set the root in the DAG. This returns a Sender<Event> for sending events.
        let parser_sender = dag.set_root(parser_node);

        // 5. Create a "sink node" that captures `BerkeleyBlock` events by storing them in a `Vec<String>`.
        let sink_node = ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|event, state, _requeue| {
                Box::pin(async move {
                    if event.event_type == EventType::BerkeleyBlock {
                        let mut locked_state = state.lock().await;
                        // Maintain a vector of payloads for all BerkeleyBlock events received
                        let mut captured_blocks: Vec<String> = locked_state.get("captured_blocks").cloned().unwrap_or_default();
                        captured_blocks.push(event.payload.clone());
                        locked_state.insert("captured_blocks", captured_blocks);
                    }
                    None
                })
            })
            .build();
        let sink_node_id = sink_node.id();

        // 6. Add the sink node to the DAG and link it to the parser node
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

        // 9. Define the path to a Berkeley block file
        let block_file = "./src/event_sourcing/test_data/berkeley_blocks/berkeley-10-3NL53c8uTVnoFjzh17VAeCR9r3zjmDowNpeFRRVEUqnvX5WdHtWE.json";

        // 10. Send a BerkeleyBlockPath event to the parser node
        let event = Event {
            event_type: EventType::BerkeleyBlockPath,
            payload: block_file.to_string(),
        };
        parser_sender.send(event).await.expect("Failed to send BerkeleyBlockPath event");

        // 11. Allow the DAG time to process
        sleep(Duration::from_millis(200)).await;

        // 12. Check the sink node's state for the `captured_blocks`
        let sink_state = {
            let dag_locked = dag.lock().await;
            let sink_node_locked = dag_locked.read_node(sink_node_id.clone()).expect("Sink node not found").lock().await;
            let state = sink_node_locked.get_state();
            let store_locked = state.lock().await;
            store_locked.get::<Vec<String>>("captured_blocks").cloned().unwrap_or_default()
        };

        // We expect exactly one BerkeleyBlock payload
        assert_eq!(sink_state.len(), 1, "Should have exactly 1 captured BerkeleyBlock event");

        // 13. Deserialize the BerkeleyBlock payload and verify fields
        let payload_str = &sink_state[0];
        let block_payload: BerkeleyBlockPayload = sonic_rs::from_str(payload_str).expect("Failed to deserialize BerkeleyBlockPayload");

        assert_eq!(block_payload.height, 10);
        assert_eq!(block_payload.state_hash, "3NL53c8uTVnoFjzh17VAeCR9r3zjmDowNpeFRRVEUqnvX5WdHtWE");
        assert_eq!(block_payload.previous_state_hash, "3NKJarZEsMAHkcPfhGA72eyjWBXGHergBZEoTuGXWS7vWeq8D5wu");
        assert_eq!(block_payload.last_vrf_output, "hu0nffAHwdL0CYQNAlabyiUlwNWhlbj0MwynpKLtAAA=");

        // 14. Signal shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }
}
