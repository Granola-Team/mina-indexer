use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    stream::{mainnet_block_models::MainnetBlock, payloads::MainnetBlockPayload},
    utility::extract_height_and_hash,
};
use async_trait::async_trait;
use std::{
    fs,
    path::Path,
    sync::{atomic::AtomicUsize, Arc},
};

pub struct MainnetBlockParserActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_processed: AtomicUsize,
}

#[async_trait]
impl Actor for MainnetBlockParserActor {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn events_processed(&self) -> &AtomicUsize {
        &self.events_processed
    }
    async fn handle_event(&self, event: Event) {
        if let EventType::MainnetBlockPath = event.event_type {
            let (height, state_hash) = extract_height_and_hash(Path::new(&event.payload));
            let file_content = fs::read_to_string(Path::new(&event.payload)).expect("Failed to read JSON file from disk");
            let block: MainnetBlock = sonic_rs::from_str(&file_content).unwrap();
            let block_payload = MainnetBlockPayload {
                height: height as u64,
                state_hash: state_hash.to_string(),
                previous_state_hash: block.get_previous_state_hash(),
                last_vrf_output: block.get_last_vrf_output(),
            };
            self.publish(Event {
                event_type: EventType::MainnetBlock,
                payload: sonic_rs::to_string(&block_payload).unwrap(),
            });
            self.incr_event_processed();
        }
    }

    fn publish(&self, event: Event) {
        self.shared_publisher.publish(event);
    }
}

#[tokio::test]
async fn test_mainnet_block_parser_actor() -> anyhow::Result<()> {
    use crate::stream::payloads::MainnetBlockPayload;
    use std::sync::atomic::Ordering;

    // Create shared publisher
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = MainnetBlockParserActor {
        id: "TestActor".to_string(),
        shared_publisher: Arc::clone(&shared_publisher),
        events_processed: AtomicUsize::new(0),
    };

    let block_file = "./src/stream/test_data/100_mainnet_blocks/mainnet-100-3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4.json";

    // Create an event pointing to the temporary file with the correct event type
    let event = Event {
        event_type: EventType::MainnetBlockPath,
        payload: block_file.to_string(),
    };

    // Subscribe to the shared publisher
    let mut receiver = shared_publisher.subscribe();

    // Invoke the actor with the MainnetBlock event
    actor.on_event(event).await;

    // Assert that the correct MainnetBlock event is published
    if let Ok(received_event) = receiver.recv().await {
        assert_eq!(received_event.event_type, EventType::MainnetBlock);

        // Deserialize the payload and check values
        let payload: MainnetBlockPayload = sonic_rs::from_str(&received_event.payload).unwrap();
        assert_eq!(payload.height, 100); // Ensure this matches extract_height_and_hash
        assert_eq!(payload.state_hash, "3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4");
        assert_eq!(payload.previous_state_hash, "3NLdywCHZmuqxS4hUnW7Uuu6sr97iifh5Ldc6m9EbzVZyLqbxqCh");
        assert_eq!(payload.last_vrf_output, "HXzRY01h73mWXp4cjNwdDTYLDtdFU5mYhTbWWi-1wwE=");
        assert_eq!(actor.events_processed().load(Ordering::SeqCst), 1);
    } else {
        panic!("Did not receive expected event from actor.");
    }

    Ok(())
}
