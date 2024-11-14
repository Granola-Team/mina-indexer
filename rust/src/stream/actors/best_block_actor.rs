use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::stream::payloads::BlockCanonicityUpdatePayload;
use async_trait::async_trait;
use futures::lock::Mutex;
use std::sync::{atomic::AtomicUsize, Arc};

#[derive(Clone)]
pub struct BestBlock {
    pub height: u64,
    pub state_hash: String,
}

pub struct BestBlockActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
    pub best_block: Arc<Mutex<Option<BestBlock>>>,
}

impl BestBlockActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "BestBlockActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
            best_block: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait]
impl Actor for BestBlockActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.events_published
    }
    async fn handle_event(&self, event: Event) {
        if let EventType::BlockCanonicityUpdate = event.event_type {
            let block_payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&event.payload).unwrap();

            let mut best_block_lock = self.best_block.lock().await;
            match &mut *best_block_lock {
                // Initialize best block if not set
                None => {
                    *best_block_lock = Some(BestBlock {
                        height: block_payload.height,
                        state_hash: block_payload.state_hash.clone(),
                    });
                    self.publish(Event {
                        event_type: EventType::BestBlock,
                        payload: event.payload,
                    });
                }
                // Update best block if the new block is canonical and has a higher height
                Some(best_block) if block_payload.canonical && block_payload.height >= best_block.height => {
                    best_block.height = block_payload.height;
                    best_block.state_hash = block_payload.state_hash;
                    self.publish(Event {
                        event_type: EventType::BestBlock,
                        payload: event.payload,
                    });
                }
                _ => {}
            }
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[tokio::test]
async fn test_best_block_actor_updates() -> anyhow::Result<()> {
    use crate::stream::payloads::BlockCanonicityUpdatePayload;
    use std::sync::atomic::Ordering;

    // Create a shared publisher and the actor
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = BestBlockActor::new(Arc::clone(&shared_publisher));

    // Subscribe to the shared publisher to capture any output events
    let mut receiver = shared_publisher.subscribe();

    // Initial best block should be None
    assert!(actor.best_block.lock().await.is_none());

    // Define a canonical block update payload with a height of 2
    let canonical_block_payload = BlockCanonicityUpdatePayload {
        height: 2,
        state_hash: "new_canonical_hash".to_string(),
        canonical: true,
    };

    // Handle the canonical block update event
    actor
        .handle_event(Event {
            event_type: EventType::BlockCanonicityUpdate,
            payload: sonic_rs::to_string(&canonical_block_payload).unwrap(),
        })
        .await;

    // Check that the best block was updated
    let best_block = actor.best_block.lock().await;
    let best_block = best_block.as_ref().expect("Best block should be set");
    assert_eq!(best_block.height, 2);
    assert_eq!(best_block.state_hash, "new_canonical_hash");

    // Check that a BestBlock event was published
    if let Ok(received_event) = receiver.recv().await {
        assert_eq!(received_event.event_type, EventType::BestBlock);
        let published_payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&received_event.payload).unwrap();
        assert_eq!(published_payload, canonical_block_payload);
    } else {
        panic!("Expected a BestBlock event but did not receive one.");
    }

    // Verify that events_published was incremented
    assert_eq!(actor.events_published.load(Ordering::SeqCst), 1);

    Ok(())
}

#[tokio::test]
async fn test_best_block_actor_does_not_publish_on_non_canonical_or_lower_height_block() -> anyhow::Result<()> {
    use crate::stream::payloads::BlockCanonicityUpdatePayload;
    use std::sync::atomic::Ordering;
    use tokio::time::{timeout, Duration};

    // Create a shared publisher and the actor
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = BestBlockActor::new(Arc::clone(&shared_publisher));

    // Subscribe to the shared publisher to capture any output events
    let mut receiver = shared_publisher.subscribe();

    // Define a canonical block to initialize the best block
    let initial_canonical_block_payload = BlockCanonicityUpdatePayload {
        height: 2,
        state_hash: "canonical_hash_2".to_string(),
        canonical: true,
    };

    // Handle the canonical block to set an initial best block
    actor
        .handle_event(Event {
            event_type: EventType::BlockCanonicityUpdate,
            payload: sonic_rs::to_string(&initial_canonical_block_payload).unwrap(),
        })
        .await;

    // Check that the best block was set to the initial canonical block
    {
        let best_block = actor.best_block.lock().await;
        let best_block = best_block.as_ref().expect("Best block should be set");
        assert_eq!(best_block.height, 2);
        assert_eq!(best_block.state_hash, "canonical_hash_2");
    }

    // Clear any initial events from the receiver
    while receiver.try_recv().is_ok() {}

    // Process a non-canonical block update with a higher height (should not update)
    let non_canonical_block_payload = BlockCanonicityUpdatePayload {
        height: 3,
        state_hash: "non_canonical_hash_3".to_string(),
        canonical: false,
    };
    actor
        .handle_event(Event {
            event_type: EventType::BlockCanonicityUpdate,
            payload: sonic_rs::to_string(&non_canonical_block_payload).unwrap(),
        })
        .await;

    // Process a canonical block update with a lower height (should not update)
    let lower_height_canonical_block_payload = BlockCanonicityUpdatePayload {
        height: 1,
        state_hash: "canonical_hash_1".to_string(),
        canonical: true,
    };
    actor
        .handle_event(Event {
            event_type: EventType::BlockCanonicityUpdate,
            payload: sonic_rs::to_string(&lower_height_canonical_block_payload).unwrap(),
        })
        .await;

    // Use timeout to confirm no events were published for the non-canonical or lower height block
    let result = timeout(Duration::from_secs(1), receiver.recv()).await;
    assert!(result.is_err(), "No event should have been published for non-canonical or lower height block.");

    // Confirm that the best block remains unchanged
    let best_block = actor.best_block.lock().await;
    let best_block = best_block.as_ref().expect("Best block should still be set");
    assert_eq!(best_block.height, 2);
    assert_eq!(best_block.state_hash, "canonical_hash_2");

    // Verify that events_published has only been incremented once
    assert_eq!(actor.events_published.load(Ordering::SeqCst), 1);

    Ok(())
}
