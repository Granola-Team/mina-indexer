use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    constants::TRANSITION_FRONTIER_DISTANCE,
    stream::{canonical_items_manager::CanonicalItemsManager, payloads::*},
};
use async_trait::async_trait;
use futures::lock::Mutex;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct SnarkCanonicitySummaryActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
    pub canonical_items_manager: Arc<Mutex<CanonicalItemsManager<SnarkCanonicitySummaryPayload>>>,
}

impl SnarkCanonicitySummaryActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "SnarkCanonicitySummaryActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
            canonical_items_manager: Arc::new(Mutex::new(CanonicalItemsManager::new((TRANSITION_FRONTIER_DISTANCE / 5usize) as u64))),
        }
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
        let manager = self.canonical_items_manager.lock().await;
        manager.report(&self.id()).await;
    }

    async fn handle_event(&self, event: Event) {
        match event.event_type {
            EventType::BlockCanonicityUpdate => {
                let payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&event.payload).unwrap();
                let manager = self.canonical_items_manager.lock().await;
                manager.add_block_canonicity_update(payload.clone()).await;
                for payload in manager.get_updates(payload.height).await.iter() {
                    self.publish(Event {
                        event_type: EventType::SnarkCanonicitySummary,
                        payload: sonic_rs::to_string(&payload).unwrap(),
                    });
                }

                if let Err(e) = manager.prune().await {
                    eprintln!("{}", e);
                }
            }
            EventType::MainnetBlock => {
                let event_payload: MainnetBlockPayload = sonic_rs::from_str(&event.payload).unwrap();
                let manager = self.canonical_items_manager.lock().await;
                manager
                    .add_items_count(event_payload.height, &event_payload.state_hash, event_payload.snark_work_count as u64)
                    .await;
            }
            EventType::SnarkWorkSummary => {
                let event_payload: SnarkWorkSummaryPayload = sonic_rs::from_str(&event.payload).unwrap();
                let manager = self.canonical_items_manager.lock().await;
                manager
                    .add_item(SnarkCanonicitySummaryPayload {
                        height: event_payload.height,
                        state_hash: event_payload.state_hash.to_string(),
                        timestamp: event_payload.timestamp,
                        prover: event_payload.prover.to_string(),
                        fee_nanomina: event_payload.fee_nanomina,
                        canonical: false, //set default value
                    })
                    .await;
                for payload in manager.get_updates(event_payload.height).await.iter() {
                    self.publish(Event {
                        event_type: EventType::SnarkCanonicitySummary,
                        payload: sonic_rs::to_string(&payload).unwrap(),
                    });
                }

                if let Err(e) = manager.prune().await {
                    eprintln!("{}", e);
                }
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
async fn test_snark_canonicity_summary_actor_with_mainnet_block() -> anyhow::Result<()> {
    use crate::stream::payloads::{BlockCanonicityUpdatePayload, MainnetBlockPayload, SnarkCanonicitySummaryPayload, SnarkWorkSummaryPayload};
    use tokio::time::timeout;

    // Helper function to create the actor
    async fn setup_actor() -> (SnarkCanonicitySummaryActor, tokio::sync::broadcast::Receiver<Event>) {
        let shared_publisher = Arc::new(SharedPublisher::new(200));
        let actor = SnarkCanonicitySummaryActor::new(Arc::clone(&shared_publisher));
        let receiver = shared_publisher.subscribe();
        (actor, receiver)
    }

    // Helper function to send an event to the actor
    async fn send_event(actor: &SnarkCanonicitySummaryActor, event: Event) {
        actor.handle_event(event).await;
    }

    // Helper function to assert published events
    async fn assert_published_event(receiver: &mut tokio::sync::broadcast::Receiver<Event>, expected_payload: &SnarkCanonicitySummaryPayload) {
        if let Ok(event) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            let published_event: SnarkCanonicitySummaryPayload = sonic_rs::from_str(&event.unwrap().payload).unwrap();

            // Compare relevant fields
            assert_eq!(published_event.height, expected_payload.height, "Mismatch in `height` field");
            assert_eq!(published_event.state_hash, expected_payload.state_hash, "Mismatch in `state_hash` field");
            assert_eq!(published_event.timestamp, expected_payload.timestamp, "Mismatch in `timestamp` field");
            assert_eq!(published_event.prover, expected_payload.prover, "Mismatch in `prover` field");
            assert_eq!(published_event.canonical, expected_payload.canonical, "Mismatch in `canonical` field");
        } else {
            panic!("Expected event was not published.");
        }
    }

    // Set up the actor and receiver
    let (actor, mut receiver) = setup_actor().await;

    // Send a MainnetBlock event to set the expected number of SnarkWorkSummary items
    let mainnet_block_event = Event {
        event_type: EventType::MainnetBlock,
        payload: sonic_rs::to_string(&MainnetBlockPayload {
            height: 10,
            state_hash: "sample_hash".to_string(),
            snark_work_count: 2,
            ..Default::default()
        })
        .unwrap(),
    };
    send_event(&actor, mainnet_block_event).await;

    // Send SnarkWorkSummary events to match the expected count
    let snark_work_events = [
        SnarkWorkSummaryPayload {
            height: 10,
            state_hash: "sample_hash".to_string(),
            timestamp: 123456,
            prover: "test_prover_1".to_string(),
            fee_nanomina: 250_000_000,
        },
        SnarkWorkSummaryPayload {
            height: 10,
            state_hash: "sample_hash".to_string(),
            timestamp: 123457,
            prover: "test_prover_2".to_string(),
            fee_nanomina: 500_000_000,
        },
    ];

    for snark in snark_work_events.iter() {
        send_event(
            &actor,
            Event {
                event_type: EventType::SnarkWorkSummary,
                payload: sonic_rs::to_string(snark).unwrap(),
            },
        )
        .await;
    }

    // Send BlockCanonicityUpdate event
    let block_update_event = Event {
        event_type: EventType::BlockCanonicityUpdate,
        payload: sonic_rs::to_string(&BlockCanonicityUpdatePayload {
            height: 10,
            canonical: true,
            state_hash: "sample_hash".to_string(),
            was_canonical: false,
        })
        .unwrap(),
    };
    send_event(&actor, block_update_event).await;

    // Validate published event
    for snark in snark_work_events.iter() {
        assert_published_event(
            &mut receiver,
            &SnarkCanonicitySummaryPayload {
                height: snark.height,
                state_hash: snark.state_hash.clone(),
                timestamp: snark.timestamp,
                prover: snark.prover.clone(),
                fee_nanomina: snark.fee_nanomina,
                canonical: true,
            },
        )
        .await;
    }

    Ok(())
}
