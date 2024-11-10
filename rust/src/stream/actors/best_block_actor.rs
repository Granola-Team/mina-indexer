use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{constants::GENESIS_STATE_HASH, stream::payloads::BlockCanonicityUpdatePayload};
use async_trait::async_trait;
use futures::lock::Mutex;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct BestBlock {
    pub height: u64,
    pub state_hash: String,
}

pub struct BestBlockActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
    pub best_block: Arc<Mutex<BestBlock>>,
}

impl BestBlockActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "BestBlockActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
            best_block: Arc::new(Mutex::new(BestBlock {
                height: 1,
                state_hash: GENESIS_STATE_HASH.to_string(),
            })),
        }
    }
}

#[async_trait]
impl Actor for BestBlockActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn events_published(&self) -> &AtomicUsize {
        &self.events_published
    }
    async fn handle_event(&self, event: Event) {
        if let EventType::BlockCanonicityUpdate = event.event_type {
            let block_payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&event.payload).unwrap();

            let mut best_block = self.best_block.lock().await;
            if block_payload.canonical && block_payload.height > best_block.height {
                best_block.height = block_payload.height;
                best_block.state_hash = block_payload.state_hash;
                self.publish(Event {
                    event_type: EventType::BestBlock,
                    payload: event.payload,
                });
            }
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[tokio::test]
async fn test_best_block_actor_updates_on_canonical_block() -> anyhow::Result<()> {
    use crate::{constants::GENESIS_STATE_HASH, stream::payloads::BlockCanonicityUpdatePayload};
    use std::sync::atomic::Ordering;

    // Create a shared publisher and the actor
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = BestBlockActor::new(Arc::clone(&shared_publisher));

    // Subscribe to the shared publisher to capture any output events
    let mut receiver = shared_publisher.subscribe();

    // Initial best block should be the genesis block
    let initial_best_block = actor.best_block.lock().await;
    assert_eq!(initial_best_block.height, 1);
    assert_eq!(initial_best_block.state_hash, GENESIS_STATE_HASH);
    drop(initial_best_block); // Release lock for the test

    // Define a canonical block update payload with a higher height
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
    assert_eq!(best_block.height, 2);
    assert_eq!(best_block.state_hash, "new_canonical_hash");

    // Check that a MainnetBlockPath event was published
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
async fn test_best_block_actor_does_not_update_on_non_canonical_block() -> anyhow::Result<()> {
    use crate::stream::payloads::BlockCanonicityUpdatePayload;

    // Create a shared publisher and the actor
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = BestBlockActor::new(Arc::clone(&shared_publisher));

    // Subscribe to the shared publisher to capture any output events
    let mut receiver = shared_publisher.subscribe();

    // Define a non-canonical block update payload with a higher height
    let non_canonical_block_payload = BlockCanonicityUpdatePayload {
        height: 2,
        state_hash: "non_canonical_hash".to_string(),
        canonical: false,
    };

    // Handle the non-canonical block update event
    actor
        .handle_event(Event {
            event_type: EventType::BlockCanonicityUpdate,
            payload: sonic_rs::to_string(&non_canonical_block_payload).unwrap(),
        })
        .await;

    // Check that the best block was not updated
    let best_block = actor.best_block.lock().await;
    assert_eq!(best_block.height, 1); // Should still be the genesis block height
    assert_eq!(best_block.state_hash, GENESIS_STATE_HASH);

    // Ensure no event was published
    assert!(receiver.try_recv().is_err(), "No event should have been published for a non-canonical block.");

    Ok(())
}
