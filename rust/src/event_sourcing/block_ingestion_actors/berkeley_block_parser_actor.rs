use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    event_sourcing::{berkeley_block_models::BerkeleyBlock, block::BlockTrait, payloads::BerkeleyBlockPayload},
    utility::extract_height_and_hash,
};
use async_trait::async_trait;
use std::{
    fs,
    path::Path,
    sync::{atomic::AtomicUsize, Arc},
};

pub struct BerkeleyBlockParserActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
}

impl BerkeleyBlockParserActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "BerkeleyBlockParserActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Actor for BerkeleyBlockParserActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.events_published
    }

    async fn handle_event(&self, event: Event) {
        if let EventType::BerkeleyBlockPath = event.event_type {
            let (height, state_hash) = extract_height_and_hash(Path::new(&event.payload));
            let file_content = fs::read_to_string(Path::new(&event.payload)).expect("Failed to read JSON file from disk");
            let berkeley_block: BerkeleyBlock = sonic_rs::from_str(&file_content).unwrap();
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
                ..Default::default()
            };
            self.publish(Event {
                event_type: EventType::BerkeleyBlock,
                payload: sonic_rs::to_string(&berkeley_block_payload).unwrap(),
            });
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[tokio::test]
async fn test_berkeley_block_parser_actor() -> anyhow::Result<()> {
    use crate::event_sourcing::payloads::BerkeleyBlockPayload;
    use std::sync::atomic::Ordering;

    // Create shared publisher
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = BerkeleyBlockParserActor {
        id: "TestActor".to_string(),
        shared_publisher: Arc::clone(&shared_publisher),
        events_published: AtomicUsize::new(0),
    };

    let block_file = "./src/event_sourcing/test_data/berkeley_blocks/berkeley-10-3NL53c8uTVnoFjzh17VAeCR9r3zjmDowNpeFRRVEUqnvX5WdHtWE.json";

    // Create event pointing to the temporary file
    let event = Event {
        event_type: EventType::BerkeleyBlockPath,
        payload: block_file.to_string(),
    };

    // Subscribe to the shared publisher
    let mut receiver = shared_publisher.subscribe();

    // Invoke the actor with the BerkeleyBlock event
    actor.on_event(event).await;

    // Assert that the correct BerkeleyBlock event is published
    if let Ok(received_event) = receiver.recv().await {
        assert_eq!(received_event.event_type, EventType::BerkeleyBlock);

        // Deserialize the payload and check values
        let payload: BerkeleyBlockPayload = sonic_rs::from_str(&received_event.payload).unwrap();
        assert_eq!(payload.height, 10); // Ensure this matches extract_height_and_hash
        assert_eq!(payload.state_hash, "3NL53c8uTVnoFjzh17VAeCR9r3zjmDowNpeFRRVEUqnvX5WdHtWE");
        assert_eq!(payload.previous_state_hash, "3NKJarZEsMAHkcPfhGA72eyjWBXGHergBZEoTuGXWS7vWeq8D5wu");
        assert_eq!(payload.last_vrf_output, "hu0nffAHwdL0CYQNAlabyiUlwNWhlbj0MwynpKLtAAA=");
        assert_eq!(actor.actor_outputs().load(Ordering::SeqCst), 1);
    } else {
        panic!("Did not receive expected event from actor.");
    }

    Ok(())
}
