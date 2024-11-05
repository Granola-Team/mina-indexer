use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
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
            self.publish(Event {
                event_type: EventType::PrecomputedBlockPath,
                payload: event.payload,
            })
        }
    }

    fn publish(&self, event: Event) {
        self.shared_publisher.publish(event);
    }
}

#[tokio::test]
async fn test_precomputed_block_path_identity_actor() {
    // Initialize shared publisher
    let shared_publisher = Arc::new(SharedPublisher::new(200));

    // Create an instance of the actor
    let actor = PCBBlockPathActor {
        id: "PCBBlockPathActor".to_string(),
        shared_publisher: Arc::clone(&shared_publisher),
    };

    // Subscribe to the shared publisher to listen for actor responses
    let mut receiver = shared_publisher.subscribe();

    // Define the test event that the actor should respond to
    let test_event = Event {
        event_type: EventType::PrecomputedBlockPath,
        payload: "/path/to/precomputed_block".to_string(),
    };

    // Send the test event to the actor
    actor.on_event(test_event).await;

    // Check that the actor publishes the expected event in response
    if let Ok(received_event) = receiver.recv().await {
        assert_eq!(received_event.event_type, EventType::PrecomputedBlockPath);
        assert_eq!(
            received_event.payload,
            "/path/to/precomputed_block".to_string()
        );
    } else {
        panic!("Did not receive expected event from actor.");
    }
}
