use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    stream::{berkeley_block_models::BerkeleyBlock, payloads::BerkeleyBlockPayload},
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
    pub events_processed: AtomicUsize,
}

impl BerkeleyBlockParserActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "BerkeleyBlockParserActor".to_string(),
            shared_publisher,
            events_processed: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Actor for BerkeleyBlockParserActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn events_processed(&self) -> &AtomicUsize {
        &self.events_processed
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
            };
            self.publish(Event {
                event_type: EventType::BerkeleyBlock,
                payload: sonic_rs::to_string(&berkeley_block_payload).unwrap(),
            });
            self.incr_event_processed();
        }
    }

    fn publish(&self, event: Event) {
        self.shared_publisher.publish(event);
    }
}

#[tokio::test]
async fn test_berkeley_block_parser_actor() -> anyhow::Result<()> {
    use crate::stream::payloads::BerkeleyBlockPayload;
    use std::sync::atomic::Ordering;

    // Create shared publisher
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = BerkeleyBlockParserActor {
        id: "TestActor".to_string(),
        shared_publisher: Arc::clone(&shared_publisher),
        events_processed: AtomicUsize::new(0),
    };

    let block_file = "./src/stream/test_data/berkeley_blocks/berkeley-10-3NL53c8uTVnoFjzh17VAeCR9r3zjmDowNpeFRRVEUqnvX5WdHtWE.json";

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
        assert_eq!(actor.events_processed().load(Ordering::SeqCst), 1);
    } else {
        panic!("Did not receive expected event from actor.");
    }

    Ok(())
}
