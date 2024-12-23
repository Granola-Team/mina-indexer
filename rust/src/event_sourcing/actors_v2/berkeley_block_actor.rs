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
use std::{fs, path::Path};
use tokio::sync::watch;

pub struct BerkeleyBlockActor;

impl ActorFactory for BerkeleyBlockActor {
    fn create_actor(shutdown_rx: watch::Receiver<bool>) -> ActorNode {
        ActorNodeBuilder::new(EventType::BerkeleyBlockPath)
            .with_state(ActorStore::new())
            .with_processor(|event, _state| {
                Box::pin(async move {
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
                    Some(Event {
                        event_type: EventType::BerkeleyBlock,
                        payload: sonic_rs::to_string(&berkeley_block_payload).unwrap(),
                    })
                })
            })
            .build(shutdown_rx)
    }
}

#[cfg(test)]
mod berkeley_block_actor_tests_v2 {
    use super::BerkeleyBlockActor;
    use crate::event_sourcing::{
        actor_dag::{ActorFactory, ActorNode},
        events::{Event, EventType},
        payloads::BerkeleyBlockPayload,
    };
    use std::sync::Arc;
    use tokio::sync::{watch, Mutex};

    #[tokio::test]
    async fn test_berkeley_block_parser_actor_with_real_files() {
        // Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Create the actor
        let mut actor = BerkeleyBlockActor::create_actor(shutdown_rx);

        // Add a receiver to the actor to capture its output
        let mut test_receiver = actor.add_receiver(EventType::BerkeleyBlock);

        // Consume the sender once and hold a reference to it
        let sender = actor.consume_sender().unwrap();

        // Wrap the actor in Arc<Mutex> for shared ownership
        let actor = Arc::new(Mutex::new(actor));

        // Spawn the actor
        tokio::spawn({
            let actor_clone = Arc::clone(&actor);
            async move {
                ActorNode::spawn_all(actor_clone).await;
            }
        });

        // Define the path to a Berkeley block file
        let block_file = "./src/event_sourcing/test_data/berkeley_blocks/berkeley-10-3NL53c8uTVnoFjzh17VAeCR9r3zjmDowNpeFRRVEUqnvX5WdHtWE.json";

        // Scenario: Test the Berkeley block
        let event = Event {
            event_type: EventType::BerkeleyBlockPath,
            payload: block_file.to_string(),
        };

        // Send the event to the actor using the held sender reference
        sender.send(event).await.expect("Failed to send event");

        // Verify the BerkeleyBlock event
        if let Some(received_event) = test_receiver.recv().await {
            assert_eq!(received_event.event_type, EventType::BerkeleyBlock);

            // Deserialize the payload and verify fields
            let payload: BerkeleyBlockPayload = sonic_rs::from_str(&received_event.payload).unwrap();
            assert_eq!(payload.height, 10);
            assert_eq!(payload.state_hash, "3NL53c8uTVnoFjzh17VAeCR9r3zjmDowNpeFRRVEUqnvX5WdHtWE");
            assert_eq!(payload.previous_state_hash, "3NKJarZEsMAHkcPfhGA72eyjWBXGHergBZEoTuGXWS7vWeq8D5wu");
            assert_eq!(payload.last_vrf_output, "hu0nffAHwdL0CYQNAlabyiUlwNWhlbj0MwynpKLtAAA=");
        } else {
            panic!("Did not receive expected event.");
        }

        // Signal shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }
}
