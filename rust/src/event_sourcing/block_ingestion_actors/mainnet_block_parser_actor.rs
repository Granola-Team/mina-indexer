use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    event_sourcing::{mainnet_block_models::MainnetBlock, payloads::MainnetBlockPayload},
    utility::{extract_height_and_hash, get_cleaned_pcb},
};
use async_trait::async_trait;
use std::{
    path::Path,
    sync::{atomic::AtomicUsize, Arc},
};

pub struct MainnetBlockParserActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
}

impl MainnetBlockParserActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "MainnetBlockParserActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Actor for MainnetBlockParserActor {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn actor_outputs(&self) -> &AtomicUsize {
        &self.events_published
    }
    async fn handle_event(&self, event: Event) {
        if let EventType::MainnetBlockPath = event.event_type {
            let (height, state_hash) = extract_height_and_hash(Path::new(&event.payload));
            let file_content = get_cleaned_pcb(&event.payload).unwrap();
            let block: MainnetBlock = sonic_rs::from_str(&file_content).unwrap();
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
            self.publish(Event {
                event_type: EventType::MainnetBlock,
                payload: sonic_rs::to_string(&block_payload).unwrap(),
            });
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[tokio::test]
async fn test_mainnet_block_parser_actor() -> anyhow::Result<()> {
    use crate::event_sourcing::payloads::MainnetBlockPayload;
    use std::sync::atomic::Ordering;

    // Create shared publisher
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = MainnetBlockParserActor {
        id: "TestActor".to_string(),
        shared_publisher: Arc::clone(&shared_publisher),
        events_published: AtomicUsize::new(0),
    };

    // Define paths for two block files
    let block_file_100 = "./src/event_sourcing/test_data/100_mainnet_blocks/mainnet-100-3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4.json";
    let block_file_99 = "./src/event_sourcing/test_data/100_mainnet_blocks/mainnet-99-3NLdywCHZmuqxS4hUnW7Uuu6sr97iifh5Ldc6m9EbzVZyLqbxqCh.json";

    // Test block 100
    let event_100 = Event {
        event_type: EventType::MainnetBlockPath,
        payload: block_file_100.to_string(),
    };

    // Subscribe to the shared publisher
    let mut receiver = shared_publisher.subscribe();

    // Invoke the actor with the MainnetBlock event for block 100
    actor.on_event(event_100).await;

    // Assert that the correct MainnetBlock event is published for block 100
    if let Ok(received_event) = receiver.recv().await {
        assert_eq!(received_event.event_type, EventType::MainnetBlock);

        // Deserialize the payload and check values for block 100
        let payload: MainnetBlockPayload = sonic_rs::from_str(&received_event.payload).unwrap();
        assert_eq!(payload.height, 100);
        assert_eq!(payload.state_hash, "3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4");
        assert_eq!(payload.previous_state_hash, "3NLdywCHZmuqxS4hUnW7Uuu6sr97iifh5Ldc6m9EbzVZyLqbxqCh");
        assert_eq!(payload.last_vrf_output, "HXzRY01h73mWXp4cjNwdDTYLDtdFU5mYhTbWWi-1wwE=");
        assert_eq!(payload.user_command_count, 1);
        assert_eq!(payload.snark_work_count, 0);
        assert_eq!(actor.actor_outputs().load(Ordering::SeqCst), 1);
    } else {
        panic!("Did not receive expected event from actor for block 100.");
    }

    // Test block 99
    let event_99 = Event {
        event_type: EventType::MainnetBlockPath,
        payload: block_file_99.to_string(),
    };

    // Invoke the actor with the MainnetBlock event for block 99
    actor.on_event(event_99).await;

    // Assert that the correct MainnetBlock event is published for block 99
    if let Ok(received_event) = receiver.recv().await {
        assert_eq!(received_event.event_type, EventType::MainnetBlock);

        // Deserialize the payload and check values for block 99
        let payload: MainnetBlockPayload = sonic_rs::from_str(&received_event.payload).unwrap();
        assert_eq!(payload.height, 99);
        assert_eq!(payload.state_hash, "3NLdywCHZmuqxS4hUnW7Uuu6sr97iifh5Ldc6m9EbzVZyLqbxqCh");
        assert_eq!(payload.previous_state_hash, "3NLAuBJPgT4Tk4LpufAEDQq4Jv9QVUefq3n3eB9x9VgGqe6LKzWp");
        assert_eq!(payload.last_vrf_output, "ws1xspEgjEyLiSS0V2-Egf9UzJG3FACpruvvDEsqDAA=");
        assert_eq!(payload.user_command_count, 3);
        assert_eq!(payload.snark_work_count, 0);
        assert_eq!(actor.actor_outputs().load(Ordering::SeqCst), 2);
    } else {
        panic!("Did not receive expected event from actor for block 99.");
    }

    Ok(())
}
