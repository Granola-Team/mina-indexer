use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    constants::TRANSITION_FRONTIER_DISTANCE,
    event_sourcing::{canonical_items_manager::CanonicalItemsManager, payloads::*},
};
use async_trait::async_trait;
use futures::lock::Mutex;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct BulkSnarkCanonicitySummaryActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
    pub canonical_items_manager: Arc<Mutex<CanonicalItemsManager<BulkSnarkCanonicityPayload>>>,
}

impl BulkSnarkCanonicitySummaryActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "BulkSnarkCanonicitySummaryActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
            canonical_items_manager: Arc::new(Mutex::new(CanonicalItemsManager::new((TRANSITION_FRONTIER_DISTANCE / 5usize) as u64))),
        }
    }
}

#[async_trait]
impl Actor for BulkSnarkCanonicitySummaryActor {
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
            EventType::MainnetBlock => {
                let event_payload: MainnetBlockPayload = sonic_rs::from_str(&event.payload).unwrap();
                let manager = self.canonical_items_manager.lock().await;

                let bulk_payload = BulkSnarkCanonicityPayload {
                    height: event_payload.height,
                    state_hash: event_payload.state_hash.to_string(),
                    canonical: true, // Default value
                    timestamp: event_payload.timestamp,
                    snarks: event_payload
                        .snark_work
                        .iter()
                        .map(|work| Snark {
                            prover: work.prover.clone(),
                            fee_nanomina: work.fee_nanomina,
                        })
                        .collect(),
                };

                manager.add_items_count(event_payload.height, &event_payload.state_hash, 1).await;
                manager.add_item(bulk_payload).await;

                // Publish the bulk event
                for payload in manager.get_updates(event_payload.height).await.iter() {
                    self.publish(Event {
                        event_type: EventType::BulkSnarkCanonicity,
                        payload: sonic_rs::to_string(&payload).unwrap(),
                    });
                }

                if let Err(e) = manager.prune().await {
                    eprintln!("{}", e);
                }
            }
            EventType::BlockCanonicityUpdate => {
                let payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&event.payload).unwrap();
                let manager = self.canonical_items_manager.lock().await;

                manager.add_block_canonicity_update(payload.clone()).await;

                // Publish bulk updates after processing block canonicity
                for payload in manager.get_updates(payload.height).await.iter() {
                    self.publish(Event {
                        event_type: EventType::BulkSnarkCanonicity,
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

#[cfg(test)]
mod bulk_snark_canonicity_summary_actor_tests {
    use super::*;
    use crate::event_sourcing::{
        events::{Event, EventType},
        mainnet_block_models::CompletedWorksNanomina,
        payloads::{BlockCanonicityUpdatePayload, BulkSnarkCanonicityPayload, MainnetBlockPayload},
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn test_publishes_after_all_conditions_met() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = BulkSnarkCanonicitySummaryActor::new(Arc::clone(&shared_publisher));
        let mut receiver = shared_publisher.subscribe();

        // Add a MainnetBlock event with snark work
        let mainnet_block = MainnetBlockPayload {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            timestamp: 1234567890,
            snark_work: vec![
                CompletedWorksNanomina {
                    prover: "prover_1".to_string(),
                    fee_nanomina: 1000,
                },
                CompletedWorksNanomina {
                    prover: "prover_2".to_string(),
                    fee_nanomina: 2000,
                },
            ],
            ..Default::default()
        };
        actor
            .handle_event(Event {
                event_type: EventType::MainnetBlock,
                payload: sonic_rs::to_string(&mainnet_block).unwrap(),
            })
            .await;

        // Add a BlockCanonicityUpdate event for the same height
        let canonicity_update = BlockCanonicityUpdatePayload {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            canonical: true,
            was_canonical: false,
        };
        actor
            .handle_event(Event {
                event_type: EventType::BlockCanonicityUpdate,
                payload: sonic_rs::to_string(&canonicity_update).unwrap(),
            })
            .await;

        // Expect the BulkSnarkCanonicity event to be published
        let event = tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv())
            .await
            .expect("Expected a published event")
            .expect("Event received");

        assert_eq!(event.event_type, EventType::BulkSnarkCanonicity);

        // Validate the payload
        let bulk_payload: BulkSnarkCanonicityPayload = sonic_rs::from_str(&event.payload).unwrap();
        assert_eq!(bulk_payload.height, 10);
        assert_eq!(bulk_payload.state_hash, "state_hash_10");
        assert!(bulk_payload.canonical);
        assert_eq!(bulk_payload.timestamp, 1234567890);
        assert_eq!(bulk_payload.snarks.len(), 2);

        let snark_1 = &bulk_payload.snarks[0];
        assert_eq!(snark_1.prover, "prover_1");
        assert_eq!(snark_1.fee_nanomina, 1000);

        let snark_2 = &bulk_payload.snarks[1];
        assert_eq!(snark_2.prover, "prover_2");
        assert_eq!(snark_2.fee_nanomina, 2000);
    }

    #[tokio::test]
    async fn test_does_not_publish_without_canonicity_update() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = BulkSnarkCanonicitySummaryActor::new(Arc::clone(&shared_publisher));
        let mut receiver = shared_publisher.subscribe();

        // Add a MainnetBlock event with snark work
        let mainnet_block = MainnetBlockPayload {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            timestamp: 1234567890,
            snark_work: vec![CompletedWorksNanomina {
                prover: "prover_1".to_string(),
                fee_nanomina: 1000,
            }],
            ..Default::default()
        };
        actor
            .handle_event(Event {
                event_type: EventType::MainnetBlock,
                payload: sonic_rs::to_string(&mainnet_block).unwrap(),
            })
            .await;

        // Expect no event to be published
        let no_event = tokio::time::timeout(std::time::Duration::from_millis(100), receiver.recv()).await;
        assert!(no_event.is_err(), "No event should be published since the canonicity update is missing");
    }

    #[tokio::test]
    async fn test_does_not_publish_without_mainnet_block() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = BulkSnarkCanonicitySummaryActor::new(Arc::clone(&shared_publisher));
        let mut receiver = shared_publisher.subscribe();

        // Add a BlockCanonicityUpdate event
        let canonicity_update = BlockCanonicityUpdatePayload {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            canonical: true,
            was_canonical: false,
        };
        actor
            .handle_event(Event {
                event_type: EventType::BlockCanonicityUpdate,
                payload: sonic_rs::to_string(&canonicity_update).unwrap(),
            })
            .await;

        // Expect no event to be published
        let no_event = tokio::time::timeout(std::time::Duration::from_millis(100), receiver.recv()).await;
        assert!(no_event.is_err(), "No event should be published since the mainnet block is missing");
    }

    #[tokio::test]
    async fn test_multiple_bulk_events_published() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = BulkSnarkCanonicitySummaryActor::new(Arc::clone(&shared_publisher));
        let mut receiver = shared_publisher.subscribe();

        // Add MainnetBlock events for multiple heights
        for i in 0..2 {
            let mainnet_block = MainnetBlockPayload {
                height: i,
                state_hash: format!("state_hash_{}", i),
                timestamp: 1234567890 + i,
                snark_work: vec![CompletedWorksNanomina {
                    prover: format!("prover_{}", i),
                    fee_nanomina: 1000 + i,
                }],
                ..Default::default()
            };
            actor
                .handle_event(Event {
                    event_type: EventType::MainnetBlock,
                    payload: sonic_rs::to_string(&mainnet_block).unwrap(),
                })
                .await;

            // Add a BlockCanonicityUpdate event for the same height
            let canonicity_update = BlockCanonicityUpdatePayload {
                height: i,
                state_hash: format!("state_hash_{}", i),
                canonical: true,
                was_canonical: false,
            };
            actor
                .handle_event(Event {
                    event_type: EventType::BlockCanonicityUpdate,
                    payload: sonic_rs::to_string(&canonicity_update).unwrap(),
                })
                .await;
        }

        // Validate that two bulk events are published
        for i in 0..2 {
            let event = tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv())
                .await
                .expect("Expected a published event")
                .expect("Event received");

            assert_eq!(event.event_type, EventType::BulkSnarkCanonicity);

            let bulk_payload: BulkSnarkCanonicityPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(bulk_payload.height, i);
            assert_eq!(bulk_payload.state_hash, format!("state_hash_{}", i));
        }
    }
}
