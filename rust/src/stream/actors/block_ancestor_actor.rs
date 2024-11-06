use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    stream::{
        berkeley_block_models::BerkeleyBlock, mainnet_block_models::MainnetBlock,
        payloads::BlockAncestorPayload,
    },
    utility::extract_height_and_hash,
};
use async_trait::async_trait;
use std::{fs, path::Path, sync::Arc};

pub struct BlockAncestorActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
}

#[async_trait]
impl Actor for BlockAncestorActor {
    fn id(&self) -> String {
        self.id.clone()
    }
    async fn on_event(&self, event: Event) {
        match event.event_type {
            EventType::BerkeleyBlockPath => {
                let (height, state_hash) = extract_height_and_hash(&Path::new(&event.payload));
                let file_content = fs::read_to_string(Path::new(&event.payload))
                    .expect("Failed to read JSON file from disk");
                let berkeley_block: BerkeleyBlock = sonic_rs::from_str(&file_content).unwrap();
                let berkeley_block_payload = BlockAncestorPayload {
                    height: height as u64,
                    state_hash: state_hash.to_string(),
                    previous_state_hash: berkeley_block.data.protocol_state.previous_state_hash,
                };
                self.publish(Event {
                    event_type: EventType::BlockAncestor,
                    payload: sonic_rs::to_string(&berkeley_block_payload).unwrap(),
                });
            }
            EventType::MainnetBlockPath => {
                let (height, state_hash) = extract_height_and_hash(&Path::new(&event.payload));
                let file_content = fs::read_to_string(Path::new(&event.payload))
                    .expect("Failed to read JSON file from disk");
                let mainnet_block: MainnetBlock = sonic_rs::from_str(&file_content).unwrap();
                let mainnet_block_payload = BlockAncestorPayload {
                    height: height as u64,
                    state_hash: state_hash.to_string(),
                    previous_state_hash: mainnet_block.protocol_state.previous_state_hash,
                };
                self.publish(Event {
                    event_type: EventType::BlockAncestor,
                    payload: sonic_rs::to_string(&mainnet_block_payload).unwrap(),
                });
            }
            _ => {}
        }
    }

    fn publish(&self, event: Event) {
        self.shared_publisher.publish(event);
    }
}

#[tokio::test]
async fn test_block_ancestor_actor_with_berkeley_block() -> anyhow::Result<()> {
    use std::io::Write;

    // Create shared publisher
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = BlockAncestorActor {
        id: "TestActor".to_string(),
        shared_publisher: Arc::clone(&shared_publisher),
    };

    // Create a temporary file for the BerkeleyBlock JSON
    let mut block_file = tempfile::Builder::new()
        .prefix("berkeley-89-3NKVkEwELHY9CmPYxf25pwsKZpPf161QVCiC3JwdsyQwCYyE3wNCrRjWON") // Updated prefix
        .suffix(".json")
        .tempfile()?;
    writeln!(
        block_file,
        r#"{{
            "version": 3,
            "data": {{
                "scheduled_time": "1706912533748",
                "protocol_state": {{
                    "previous_state_hash": "3NKJarZEsMAHkcPfhGA72eyjWBXGHergBZEoTuGXWS7vWeq8D5wu",
                    "body": {{
                        "genesis_state_hash": "3..."
                    }}
                }}
            }}
        }}"#
    )
    .unwrap();

    // Create event pointing to the temporary file
    let event = Event {
        event_type: EventType::PrecomputedBlockPath,
        payload: block_file.path().to_str().unwrap().to_string(),
    };

    // Subscribe to the shared publisher
    let mut receiver = shared_publisher.subscribe();

    // Invoke the actor with the BerkeleyBlock event
    actor.on_event(event).await;

    // Assert that the correct BlockAncestor event is published
    if let Ok(received_event) = receiver.recv().await {
        assert_eq!(received_event.event_type, EventType::BlockAncestor);

        // Deserialize the payload and check values
        let payload: BlockAncestorPayload = sonic_rs::from_str(&received_event.payload).unwrap();
        assert_eq!(payload.height, 89); // Ensure this matches extract_height_and_hash
        assert!(payload
            .state_hash
            .contains("3NKVkEwELHY9CmPYxf25pwsKZpPf161QVCiC3JwdsyQwCYyE3wNCrRjWON"));
        assert_eq!(
            payload.previous_state_hash,
            "3NKJarZEsMAHkcPfhGA72eyjWBXGHergBZEoTuGXWS7vWeq8D5wu"
        );
    } else {
        panic!("Did not receive expected event from actor.");
    }

    Ok(())
}

#[tokio::test]
async fn test_block_ancestor_actor_with_mainnet_block() -> anyhow::Result<()> {
    use std::io::Write;

    // Create shared publisher
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = BlockAncestorActor {
        id: "TestActor".to_string(),
        shared_publisher: Arc::clone(&shared_publisher),
    };

    // Create a temporary file for the MainnetBlock JSON
    let mut block_file = tempfile::Builder::new()
        .prefix("mainnet-45-3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPwgf9yam") // Updated prefix
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

    // Create event pointing to the temporary file
    let event = Event {
        event_type: EventType::MainnetBlockPath,
        payload: block_file.path().to_str().unwrap().to_string(),
    };

    // Subscribe to the shared publisher
    let mut receiver = shared_publisher.subscribe();

    // Invoke the actor with the MainnetBlock event
    actor.on_event(event).await;

    // Assert that the correct BlockAncestor event is published
    if let Ok(received_event) = receiver.recv().await {
        assert_eq!(received_event.event_type, EventType::BlockAncestor);

        // Deserialize the payload and check values
        let payload: BlockAncestorPayload = sonic_rs::from_str(&received_event.payload).unwrap();
        assert_eq!(payload.height, 45); // Ensure this matches extract_height_and_hash
        assert!(payload
            .state_hash
            .contains("3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPwgf9yam"));
        assert_eq!(
            payload.previous_state_hash,
            "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw"
        );
    } else {
        panic!("Did not receive expected event from actor.");
    }

    Ok(())
}
