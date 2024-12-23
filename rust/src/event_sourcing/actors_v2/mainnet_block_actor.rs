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
use std::path::Path;
use tokio::sync::watch;

pub struct MainnetBlockParserActor;

impl ActorFactory for MainnetBlockParserActor {
    fn create_actor(shutdown_rx: watch::Receiver<bool>) -> ActorNode {
        ActorNodeBuilder::new(EventType::MainnetBlockPath)
            .with_state(ActorStore::new())
            .with_processor(|event, _state| {
                Box::pin(async move {
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
                    Some(Event {
                        event_type: EventType::MainnetBlock,
                        payload: sonic_rs::to_string(&block_payload).unwrap(),
                    })
                })
            })
            .build(shutdown_rx)
    }
}

#[cfg(test)]
mod mainnet_block_actor_tests {
    use super::MainnetBlockParserActor;
    use crate::event_sourcing::{
        actor_dag::{ActorFactory, ActorNode},
        events::{Event, EventType},
        payloads::MainnetBlockPayload,
    };
    use std::sync::Arc;
    use tokio::sync::{watch, Mutex};

    #[tokio::test]
    async fn test_mainnet_block_parser_actor_with_real_files() {
        // Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Create the actor
        let mut actor = MainnetBlockParserActor::create_actor(shutdown_rx);

        // Add a receiver to the actor to capture its output
        let mut test_receiver = actor.add_receiver(EventType::MainnetBlock);

        // Consume the sender once and hold a reference to it
        let sender = actor.consume_sender().unwrap();

        // Spawn the actor
        tokio::spawn({
            let actor_clone = Arc::new(Mutex::new(actor));
            async move { ActorNode::spawn_all(actor_clone).await }
        });

        // Define paths for two block files
        let block_file_100 = "./src/event_sourcing/test_data/100_mainnet_blocks/mainnet-100-3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4.json";
        let block_file_99 = "./src/event_sourcing/test_data/100_mainnet_blocks/mainnet-99-3NLdywCHZmuqxS4hUnW7Uuu6sr97iifh5Ldc6m9EbzVZyLqbxqCh.json";

        // Scenario 1: Test block 100
        let event_100 = Event {
            event_type: EventType::MainnetBlockPath,
            payload: block_file_100.to_string(),
        };

        // Send the event to the actor using the held sender reference
        sender.send(event_100).await.expect("Failed to send event for block 100");

        // Verify the MainnetBlock event for block 100
        if let Some(received_event) = test_receiver.recv().await {
            assert_eq!(received_event.event_type, EventType::MainnetBlock);

            // Deserialize the payload and verify fields
            let payload: MainnetBlockPayload = sonic_rs::from_str(&received_event.payload).unwrap();
            assert_eq!(payload.height, 100);
            assert_eq!(payload.state_hash, "3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4");
            assert_eq!(payload.previous_state_hash, "3NLdywCHZmuqxS4hUnW7Uuu6sr97iifh5Ldc6m9EbzVZyLqbxqCh");
            assert_eq!(payload.last_vrf_output, "HXzRY01h73mWXp4cjNwdDTYLDtdFU5mYhTbWWi-1wwE=");
            assert_eq!(payload.user_command_count, 1);
            assert_eq!(payload.snark_work_count, 0);
        } else {
            panic!("Did not receive expected event for block 100.");
        }

        // Scenario 2: Test block 99
        let event_99 = Event {
            event_type: EventType::MainnetBlockPath,
            payload: block_file_99.to_string(),
        };

        // Send the event to the actor using the held sender reference
        sender.send(event_99).await.expect("Failed to send event for block 99");

        // Verify the MainnetBlock event for block 99
        if let Some(received_event) = test_receiver.recv().await {
            assert_eq!(received_event.event_type, EventType::MainnetBlock);

            // Deserialize the payload and verify fields
            let payload: MainnetBlockPayload = sonic_rs::from_str(&received_event.payload).unwrap();
            assert_eq!(payload.height, 99);
            assert_eq!(payload.state_hash, "3NLdywCHZmuqxS4hUnW7Uuu6sr97iifh5Ldc6m9EbzVZyLqbxqCh");
            assert_eq!(payload.previous_state_hash, "3NLAuBJPgT4Tk4LpufAEDQq4Jv9QVUefq3n3eB9x9VgGqe6LKzWp");
            assert_eq!(payload.last_vrf_output, "ws1xspEgjEyLiSS0V2-Egf9UzJG3FACpruvvDEsqDAA=");
            assert_eq!(payload.user_command_count, 3);
            assert_eq!(payload.snark_work_count, 0);
        } else {
            panic!("Did not receive expected event for block 99.");
        }

        // Signal shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }
}
