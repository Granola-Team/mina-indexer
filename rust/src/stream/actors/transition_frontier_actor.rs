use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{blockchain_tree::Height, constants::TRANSITION_FRONTIER_DISTANCE, stream::payloads::BlockCanonicityUpdatePayload};
use async_trait::async_trait;
use futures::lock::Mutex;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct TransitionFrontierActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
    pub transition_frontier: Arc<Mutex<Option<Height>>>,
}

impl TransitionFrontierActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "TransitionFrontierActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
            transition_frontier: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait]
impl Actor for TransitionFrontierActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.events_published
    }
    async fn handle_event(&self, event: Event) {
        fn get_transition_frontier(payload: &BlockCanonicityUpdatePayload) -> u64 {
            payload.height - TRANSITION_FRONTIER_DISTANCE as u64
        }
        if event.event_type == EventType::BestBlock {
            let payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&event.payload).unwrap();
            let mut transition_frontier = self.transition_frontier.lock().await;

            match &mut *transition_frontier {
                Some(tf) if payload.height > tf.0 + TRANSITION_FRONTIER_DISTANCE as u64 => {
                    tf.0 = get_transition_frontier(&payload);
                }
                None if payload.height > TRANSITION_FRONTIER_DISTANCE as u64 => {
                    *transition_frontier = Some(Height(get_transition_frontier(&payload)));
                }
                _ => return, // Early return if no action is needed
            }
            self.publish(Event {
                event_type: EventType::TransitionFrontier,
                payload: get_transition_frontier(&payload).to_string(),
            });
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[tokio::test]
async fn test_transition_frontier_actor_updates() -> anyhow::Result<()> {
    use crate::stream::payloads::BlockCanonicityUpdatePayload;
    use std::sync::atomic::Ordering;

    // Create a shared publisher and the actor
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = TransitionFrontierActor::new(Arc::clone(&shared_publisher));

    // Subscribe to the shared publisher to capture output events
    let mut receiver = shared_publisher.subscribe();

    // Initial transition frontier should be None
    assert!(actor.transition_frontier.lock().await.is_none());

    // Define a BestBlock event payload with height above TRANSITION_FRONTIER_DISTANCE
    let payload = BlockCanonicityUpdatePayload {
        height: TRANSITION_FRONTIER_DISTANCE as u64 + 1,
        state_hash: "some_hash".to_string(),
        canonical: true,
        was_canonical: false,
    };

    // Send the BestBlock event
    actor
        .handle_event(Event {
            event_type: EventType::BestBlock,
            payload: sonic_rs::to_string(&payload).unwrap(),
        })
        .await;

    // Verify that the transition frontier was set correctly
    let transition_frontier = actor.transition_frontier.lock().await;
    let transition_frontier = transition_frontier.as_ref().expect("Transition frontier should be set");
    assert_eq!(transition_frontier.0, payload.height - TRANSITION_FRONTIER_DISTANCE as u64);

    // Confirm that a TransitionFrontier event was published
    if let Ok(received_event) = receiver.recv().await {
        assert_eq!(received_event.event_type, EventType::TransitionFrontier);
        assert_eq!(received_event.payload, (payload.height - TRANSITION_FRONTIER_DISTANCE as u64).to_string());
    } else {
        panic!("Expected a TransitionFrontier event but did not receive one.");
    }

    // Verify that events_published was incremented
    assert_eq!(actor.events_published.load(Ordering::SeqCst), 1);

    Ok(())
}

#[tokio::test]
async fn test_transition_frontier_actor_no_update_on_lower_height() -> anyhow::Result<()> {
    use crate::stream::payloads::BlockCanonicityUpdatePayload;
    use std::sync::atomic::Ordering;
    use tokio::time::Duration;

    // Create a shared publisher and the actor
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = TransitionFrontierActor::new(Arc::clone(&shared_publisher));

    // Subscribe to the shared publisher to capture output events
    let mut receiver = shared_publisher.subscribe();

    // Initialize transition frontier with a specific height
    let initial_payload = BlockCanonicityUpdatePayload {
        height: TRANSITION_FRONTIER_DISTANCE as u64 + 10,
        state_hash: "initial_hash".to_string(),
        canonical: true,
        was_canonical: false,
    };

    // Send initial BestBlock event to set the transition frontier
    actor
        .handle_event(Event {
            event_type: EventType::BestBlock,
            payload: sonic_rs::to_string(&initial_payload).unwrap(),
        })
        .await;

    // Clear any initial events from the receiver
    while receiver.try_recv().is_ok() {}

    // Send a BestBlock event with a lower height (should not trigger an update)
    let lower_height_payload = BlockCanonicityUpdatePayload {
        height: TRANSITION_FRONTIER_DISTANCE as u64 + 5,
        state_hash: "lower_hash".to_string(),
        canonical: true,
        was_canonical: false,
    };
    tokio::time::sleep(Duration::from_secs(1)).await;
    actor
        .handle_event(Event {
            event_type: EventType::BestBlock,
            payload: sonic_rs::to_string(&lower_height_payload).unwrap(),
        })
        .await;

    // Confirm events_published was incremented only once
    assert_eq!(actor.events_published.load(Ordering::SeqCst), 1);

    Ok(())
}
