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
use std::{fs, path::Path, sync::Arc};

pub struct MainnetBlockParserActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
}

#[async_trait]
impl Actor for MainnetBlockParserActor {
    fn id(&self) -> String {
        self.id.clone()
    }
    async fn on_event(&self, event: Event) {
        if let EventType::MainnetBlock = event.event_type {
            let (height, state_hash) = extract_height_and_hash(&Path::new(&event.payload));
            let file_content = fs::read_to_string(Path::new(&event.payload))
                .expect("Failed to read JSON file from disk");
            let block: MainnetBlock = sonic_rs::from_str(&file_content).unwrap();
            let block_payload = MainnetBlockPayload {
                height: height as u64,
                state_hash: state_hash.to_string(),
                previous_state_hash: block.get_previous_state_hash(),
            };
            self.publish(Event {
                event_type: EventType::MainnetBlock,
                payload: sonic_rs::to_string(&block_payload).unwrap(),
            });
        }
    }

    fn publish(&self, event: Event) {
        self.shared_publisher.publish(event);
    }
}

#[tokio::test]
async fn test_mainnet_block_parser_actor() -> anyhow::Result<()> {
    use crate::stream::payloads::MainnetBlockPayload;
    use std::io::Write;

    // Create shared publisher
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = MainnetBlockParserActor {
        id: "TestActor".to_string(),
        shared_publisher: Arc::clone(&shared_publisher),
    };

    // Create a temporary file for the MainnetBlock JSON
    let mut block_file = tempfile::Builder::new()
        .prefix("mainnet-89-3NKVkEwELHY9CmPYxf25pwsKZpPf161QVCiC3JwdsyQwCYyE3wNCrRjWON") // Updated prefix
        .suffix(".json")
        .tempfile()?;
    writeln!(
        block_file,
        r#"{{
            "scheduled_time": "1615940848214",
            "protocol_state": {{
                "previous_state_hash": "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw",
                "body": {{
                    "genesis_state_hash": "3..."
                }}
            }}
        }}"#
    )
    .unwrap();

    // Create an event pointing to the temporary file with the correct event type
    let event = Event {
        event_type: EventType::MainnetBlock,
        payload: block_file.path().to_str().unwrap().to_string(),
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
        assert_eq!(payload.height, 89); // Ensure this matches extract_height_and_hash
        assert!(payload
            .state_hash
            .contains("3NKVkEwELHY9CmPYxf25pwsKZpPf161QVCiC3JwdsyQwCYyE3wNCrRjWON"));
        assert_eq!(
            payload.previous_state_hash,
            "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw"
        );
    } else {
        panic!("Did not receive expected event from actor.");
    }

    Ok(())
}
