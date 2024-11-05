use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::utility::get_top_level_keys_from_json_file;
use async_trait::async_trait;
use std::sync::Arc;

pub struct PCBBlockPathActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
}

#[async_trait]
impl Actor for PCBBlockPathActor {
    fn id(&self) -> String {
        self.id.clone()
    }
    async fn on_event(&self, event: Event) {
        if let EventType::PrecomputedBlockPath = event.event_type {
            let keys = get_top_level_keys_from_json_file(&event.payload).expect("file to exist");
            if keys == vec!["data".to_string(), "version".to_string()] {
                self.publish(Event {
                    event_type: EventType::BerkeleyBlockPath,
                    payload: event.payload.clone(),
                });
            } else {
                self.publish(Event {
                    event_type: EventType::MainnetBlockPath,
                    payload: event.payload,
                });
            }
        }
    }

    fn publish(&self, event: Event) {
        self.shared_publisher.publish(event);
    }
}

#[tokio::test]
async fn test_precomputed_block_path_identity_actor() {
    use tempfile::NamedTempFile;
    // Initialize shared publisher
    let shared_publisher = Arc::new(SharedPublisher::new(200));

    // Create an instance of the actor
    let actor = PCBBlockPathActor {
        id: "PCBBlockPathActor".to_string(),
        shared_publisher: Arc::clone(&shared_publisher),
    };

    // Subscribe to the shared publisher to listen for actor responses
    let mut receiver = shared_publisher.subscribe();

    // Scenario 1: File with "data" and "version" keys (should trigger BerkeleyBlockPath)
    let temp_file_berkeley = NamedTempFile::new().unwrap();
    std::fs::write(
        temp_file_berkeley.path(),
        r#"{"data": {}, "version": "1.0"}"#,
    )
    .unwrap();
    let berkeley_event = Event {
        event_type: EventType::PrecomputedBlockPath,
        payload: temp_file_berkeley.path().to_str().unwrap().to_string(),
    };
    actor.on_event(berkeley_event).await;

    // Check that the actor publishes a BerkeleyBlockPath event
    if let Ok(received_event) = receiver.recv().await {
        assert_eq!(received_event.event_type, EventType::BerkeleyBlockPath);
        assert_eq!(
            received_event.payload,
            temp_file_berkeley.path().to_str().unwrap().to_string()
        );
    } else {
        panic!("Did not receive expected BerkeleyBlockPath event from actor.");
    }

    // Scenario 2: File with different keys (should trigger MainnetBlockPath)
    let temp_file_mainnet = NamedTempFile::new().unwrap();
    std::fs::write(
        temp_file_mainnet.path(),
        r#"{"other_key": {}, "another_key": "1.0"}"#,
    )
    .unwrap();
    let mainnet_event = Event {
        event_type: EventType::PrecomputedBlockPath,
        payload: temp_file_mainnet.path().to_str().unwrap().to_string(),
    };
    actor.on_event(mainnet_event).await;

    // Check that the actor publishes a MainnetBlockPath event
    if let Ok(received_event) = receiver.recv().await {
        assert_eq!(received_event.event_type, EventType::MainnetBlockPath);
        assert_eq!(
            received_event.payload,
            temp_file_mainnet.path().to_str().unwrap().to_string()
        );
    } else {
        panic!("Did not receive expected MainnetBlockPath event from actor.");
    }
}
