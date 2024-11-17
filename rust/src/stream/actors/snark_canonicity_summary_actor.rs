use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::stream::{models::Height, payloads::*};
use anyhow::Result;
use async_trait::async_trait;
use futures::lock::Mutex;
use std::{
    collections::{HashMap, VecDeque},
    sync::{atomic::AtomicUsize, Arc},
};

pub struct SnarkCanonicitySummaryActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
    pub block_canonicity_queue: Arc<Mutex<VecDeque<BlockCanonicityUpdatePayload>>>,
    pub snark_map: Arc<Mutex<HashMap<Height, Vec<SnarkWorkSummaryPayload>>>>,
}

#[allow(dead_code)]
impl SnarkCanonicitySummaryActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "SnarkCanonicitySummaryActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
            block_canonicity_queue: Arc::new(Mutex::new(VecDeque::new())),
            snark_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn process_snark_summary(&self) -> Result<(), &'static str> {
        let mut queue = self.block_canonicity_queue.lock().await;

        while let Some(update) = queue.pop_front() {
            let snarks = self.snark_map.lock().await;
            if let Some(entries) = snarks.get(&Height(update.height)) {
                for entry in entries.iter().filter(|s| s.state_hash == update.state_hash) {
                    let payload = SnarkCanonicitySummaryPayload {
                        canonical: update.canonical,
                        height: entry.height,
                        state_hash: entry.state_hash.to_string(),
                        timestamp: entry.timestamp,
                        prover: entry.prover.to_string(),
                        fee: entry.fee,
                    };
                    self.publish(Event {
                        event_type: EventType::SnarkCanonicitySummary,
                        payload: sonic_rs::to_string(&payload).unwrap(),
                    });
                }
                queue.retain(|key| key.height > update.height); // other
            } else {
                queue.push_back(update);
                drop(queue);
                break;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Actor for SnarkCanonicitySummaryActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.events_published
    }

    async fn report(&self) {
        let snarks_map = self.snark_map.lock().await;
        self.print_report("Internal Commands HashMap", snarks_map.len());
        drop(snarks_map);
        let canonicity = self.block_canonicity_queue.lock().await;
        self.print_report("Block Canonicity Queue", canonicity.len());
    }

    async fn handle_event(&self, event: Event) {
        match event.event_type {
            EventType::BlockCanonicityUpdate => {
                let mut queue = self.block_canonicity_queue.lock().await;
                queue.push_back(sonic_rs::from_str(&event.payload).unwrap());
                drop(queue);
                self.process_snark_summary().await.expect("Expected to published snark canonicity updates");
            }
            EventType::SnarkWorkSummary => {
                let event_payload: SnarkWorkSummaryPayload = sonic_rs::from_str(&event.payload).unwrap();
                let mut snarks = self.snark_map.lock().await;
                snarks.entry(Height(event_payload.height)).or_insert_with(Vec::new).push(event_payload);
                drop(snarks);
                self.process_snark_summary().await.expect("Expected to published snark canonicity updates");
            }
            EventType::TransitionFrontier => {
                let height: u64 = sonic_rs::from_str(&event.payload).unwrap();
                let mut snarks = self.snark_map.lock().await;
                snarks.retain(|key, _| key.0 > height);
                drop(snarks);
            }
            _ => return,
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[tokio::test]
async fn test_snark_summary_persistence_actor_processes_snark_summary() -> anyhow::Result<()> {
    use crate::stream::payloads::{BlockCanonicityUpdatePayload, SnarkCanonicitySummaryPayload, SnarkWorkSummaryPayload};
    use std::sync::atomic::Ordering;
    use tokio::time::timeout;

    // Set up a shared publisher and instantiate the actor
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = SnarkCanonicitySummaryActor::new(Arc::clone(&shared_publisher));

    // Subscribe to capture any output events from the actor
    let mut receiver = shared_publisher.subscribe();

    // Insert a sample snark work summary into the snark map to be processed
    let snark_payload = SnarkWorkSummaryPayload {
        height: 10,
        state_hash: "sample_hash".to_string(),
        timestamp: 123456,
        prover: "test_prover".to_string(),
        fee: 0.25,
    };
    let snark_payload_2 = SnarkWorkSummaryPayload {
        height: 10,
        state_hash: "other_hash".to_string(),
        timestamp: 123456,
        prover: "test_prover".to_string(),
        fee: 0.25,
    };

    // Insert the snark work summary payload into the actor's snark map
    {
        let mut snarks = actor.snark_map.lock().await;
        snarks.entry(Height(snark_payload.height)).or_insert_with(Vec::new).push(snark_payload.clone());
        snarks
            .entry(Height(snark_payload_2.height))
            .or_insert_with(Vec::new)
            .push(snark_payload_2.clone());
    }

    // Send a canonical block update to trigger processing
    let canonical_update_payload = BlockCanonicityUpdatePayload {
        height: 10,
        canonical: true,
        state_hash: "sample_hash".to_string(),
        was_canonical: false,
    };

    actor
        .handle_event(Event {
            event_type: EventType::BlockCanonicityUpdate,
            payload: sonic_rs::to_string(&canonical_update_payload).unwrap(),
        })
        .await;

    // Confirm that the SnarkCanonicitySummary event was published with correct data
    let published_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
    assert!(published_event.is_ok(), "Expected a SnarkCanonicitySummary event to be published.");

    if let Ok(Ok(event)) = published_event {
        let published_payload: SnarkCanonicitySummaryPayload = sonic_rs::from_str(&event.payload).unwrap();
        assert_eq!(published_payload.height, snark_payload.height);
        assert_eq!(published_payload.state_hash, snark_payload.state_hash);
        assert_eq!(published_payload.timestamp, snark_payload.timestamp);
        assert_eq!(published_payload.prover, snark_payload.prover);
        assert_eq!(published_payload.fee, snark_payload.fee);
        assert!(published_payload.canonical);
    }

    // Verify that events_published has been incremented
    assert_eq!(actor.events_published.load(Ordering::SeqCst), 1);

    Ok(())
}

#[tokio::test]
async fn test_snark_canonicity_summary_actor_prunes_snarks_on_transition_frontier() -> anyhow::Result<()> {
    use crate::stream::payloads::SnarkWorkSummaryPayload;

    // Set up the actor
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = SnarkCanonicitySummaryActor::new(Arc::clone(&shared_publisher));

    // Insert snark work summaries with different heights
    {
        let mut snarks = actor.snark_map.lock().await;
        snarks.insert(
            Height(5),
            vec![SnarkWorkSummaryPayload {
                height: 5,
                state_hash: "hash_5".to_string(),
                timestamp: 1000,
                prover: "prover_a".to_string(),
                fee: 0.5,
            }],
        );
        snarks.insert(
            Height(10),
            vec![SnarkWorkSummaryPayload {
                height: 10,
                state_hash: "hash_10".to_string(),
                timestamp: 2000,
                prover: "prover_b".to_string(),
                fee: 0.25,
            }],
        );
    }

    // Trigger a TransitionFrontier event with height = 7 to prune snarks with height <= 7
    let transition_event = Event {
        event_type: EventType::TransitionFrontier,
        payload: sonic_rs::to_string(&7u64).unwrap(),
    };
    actor.handle_event(transition_event).await;

    // Verify that snarks with height <= 7 were removed
    {
        let snarks = actor.snark_map.lock().await;
        assert!(!snarks.contains_key(&Height(5)), "Snark with height 5 should have been pruned");
        assert!(snarks.contains_key(&Height(10)), "Snark with height 10 should not have been pruned");
    }

    Ok(())
}

#[tokio::test]
async fn test_snark_canonicity_summary_actor_defers_processing_until_snark_summary_arrives() -> anyhow::Result<()> {
    use crate::stream::payloads::{BlockCanonicityUpdatePayload, SnarkWorkSummaryPayload};
    use tokio::time::timeout;

    // Set up shared publisher and the actor
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = SnarkCanonicitySummaryActor::new(Arc::clone(&shared_publisher));

    // Subscribe to capture any output events from the actor
    let mut receiver = shared_publisher.subscribe();

    // Send a BlockCanonicityUpdate without corresponding SnarkWorkSummary, it should be deferred
    let deferred_update_payload = BlockCanonicityUpdatePayload {
        height: 10,
        canonical: true,
        state_hash: "state_hash1".to_string(),
        was_canonical: false,
    };
    actor
        .handle_event(Event {
            event_type: EventType::BlockCanonicityUpdate,
            payload: sonic_rs::to_string(&deferred_update_payload).unwrap(),
        })
        .await;

    // Use a timeout to check if any event was published (shouldn't publish as snark summary is missing)
    let result = timeout(std::time::Duration::from_millis(100), receiver.recv()).await;
    assert!(result.is_err(), "No event should have been published since SnarkWorkSummary is missing");

    // Now add the corresponding SnarkWorkSummaryPayload and verify processing
    let snark_payload = SnarkWorkSummaryPayload {
        height: 10,
        state_hash: "state_hash1".to_string(),
        timestamp: 123456,
        prover: "test_prover".to_string(),
        fee: 0.25,
    };

    // Insert the snark work summary payload into the actor's snark map
    {
        let mut snarks = actor.snark_map.lock().await;
        snarks.entry(Height(snark_payload.height)).or_insert_with(Vec::new).push(snark_payload.clone());
    }

    // Re-trigger processing by sending another canonical update
    actor
        .handle_event(Event {
            event_type: EventType::BlockCanonicityUpdate,
            payload: sonic_rs::to_string(&deferred_update_payload).unwrap(),
        })
        .await;

    // Confirm that the SnarkCanonicitySummary event was published now that snark summary is present
    let published_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
    assert!(
        published_event.is_ok(),
        "Expected a SnarkCanonicitySummary event to be published after summary is present."
    );

    Ok(())
}
