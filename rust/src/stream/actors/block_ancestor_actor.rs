use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::stream::payloads::{BerkeleyBlockPayload, BlockAncestorPayload, MainnetBlockPayload};
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct BlockAncestorActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_processed: AtomicUsize,
}

#[async_trait]
impl Actor for BlockAncestorActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn events_processed(&self) -> &AtomicUsize {
        &self.events_processed
    }
    async fn handle_event(&self, event: Event) {
        match event.event_type {
            EventType::BerkeleyBlock => {
                let block_payload: BerkeleyBlockPayload = sonic_rs::from_str(&event.payload).unwrap();
                let block_ancestor_payload = BlockAncestorPayload {
                    height: block_payload.height,
                    state_hash: block_payload.state_hash.clone(),
                    previous_state_hash: block_payload.previous_state_hash.clone(),
                    last_vrf_output: block_payload.last_vrf_output,
                };
                self.publish(Event {
                    event_type: EventType::BlockAncestor,
                    payload: sonic_rs::to_string(&block_ancestor_payload).unwrap(),
                });
                self.incr_event_processed();
            }
            EventType::MainnetBlock => {
                let block_payload: MainnetBlockPayload = sonic_rs::from_str(&event.payload).unwrap();
                let block_ancestor_payload = BlockAncestorPayload {
                    height: block_payload.height,
                    state_hash: block_payload.state_hash.clone(),
                    previous_state_hash: block_payload.previous_state_hash.clone(),
                    last_vrf_output: block_payload.last_vrf_output,
                };
                self.publish(Event {
                    event_type: EventType::BlockAncestor,
                    payload: sonic_rs::to_string(&block_ancestor_payload).unwrap(),
                });
                self.incr_event_processed();
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
    use std::sync::atomic::Ordering;
    // Create shared publisher
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = BlockAncestorActor {
        id: "TestActor".to_string(),
        shared_publisher: Arc::clone(&shared_publisher),
        events_processed: AtomicUsize::new(0),
    };

    // Define BerkeleyBlockPayload for the test
    let berkeley_block_payload = BerkeleyBlockPayload {
        height: 89,
        state_hash: "3NKVkEwELHY9CmPYxf25pwsKZpPf161QVCiC3JwdsyQwCYyE3wNCrRjWON".to_string(),
        previous_state_hash: "3NKJarZEsMAHkcPfhGA72eyjWBXGHergBZEoTuGXWS7vWeq8D5wu".to_string(),
        last_vrf_output: "hu0nffAHwdL0CYQNAlabyiUlwNWhlbj0MwynpKLtAAA=".to_string(),
    };

    // Create an Event with serialized BerkeleyBlockPayload
    let event = Event {
        event_type: EventType::BerkeleyBlock,
        payload: sonic_rs::to_string(&berkeley_block_payload).unwrap(),
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
        assert_eq!(payload.height, 89);
        assert_eq!(payload.state_hash, "3NKVkEwELHY9CmPYxf25pwsKZpPf161QVCiC3JwdsyQwCYyE3wNCrRjWON");
        assert_eq!(payload.previous_state_hash, "3NKJarZEsMAHkcPfhGA72eyjWBXGHergBZEoTuGXWS7vWeq8D5wu");
        assert_eq!(actor.events_processed().load(Ordering::SeqCst), 1);
    } else {
        panic!("Did not receive expected event from actor.");
    }

    Ok(())
}

#[tokio::test]
async fn test_block_ancestor_actor_with_mainnet_block() -> anyhow::Result<()> {
    use std::sync::atomic::Ordering;
    // Create shared publisher
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = BlockAncestorActor {
        id: "TestActor".to_string(),
        shared_publisher: Arc::clone(&shared_publisher),
        events_processed: AtomicUsize::new(0),
    };

    // Define MainnetBlockPayload for the test
    let mainnet_block_payload = MainnetBlockPayload {
        height: 101,
        state_hash: "4MTNpwef32H67dHk9Mx25ZLpHfVz27QXECm8C4o5eyRa5LgJ1qLScCwpJM".to_string(),
        previous_state_hash: "4MPXcYhJY8URpwZxBEmv9C7kXf5h41PLXeX9GoTwFg3TuL2Q9zMn".to_string(),
        last_vrf_output: "WXPOLoGn9vE7HwqkE-K5bH4d3LmSPPJQcfoLsrTDkQA=".to_string(),
    };

    // Create an Event with serialized MainnetBlockPayload
    let event = Event {
        event_type: EventType::MainnetBlock,
        payload: sonic_rs::to_string(&mainnet_block_payload).unwrap(),
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
        assert_eq!(payload.height, 101);
        assert_eq!(payload.state_hash, "4MTNpwef32H67dHk9Mx25ZLpHfVz27QXECm8C4o5eyRa5LgJ1qLScCwpJM");
        assert_eq!(payload.previous_state_hash, "4MPXcYhJY8URpwZxBEmv9C7kXf5h41PLXeX9GoTwFg3TuL2Q9zMn");
        assert_eq!(actor.events_processed().load(Ordering::SeqCst), 1);
    } else {
        panic!("Did not receive expected event from actor.");
    }

    Ok(())
}
