use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    constants::POSTGRES_CONNECTION_STRING,
    stream::{db_logger::DbLogger, payloads::*},
};
use anyhow::Result;
use async_trait::async_trait;
use futures::lock::Mutex;
use std::sync::{atomic::AtomicUsize, Arc};
use tokio_postgres::NoTls;

pub struct SnarkSummaryPersistenceActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub database_inserts: AtomicUsize,
    pub db_logger: Arc<Mutex<DbLogger>>,
}

impl SnarkSummaryPersistenceActor {
    pub async fn new(shared_publisher: Arc<SharedPublisher>, root_node: &Option<(u64, String)>) -> Self {
        if let Ok((client, connection)) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls).await {
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {}", e);
                }
            });

            let logger = DbLogger::builder(client)
                .name("snarks")
                .add_column("height BIGINT NOT NULL")
                .add_column("state_hash TEXT NOT NULL")
                .add_column("timestamp BIGINT NOT NULL")
                .add_column("prover TEXT NOT NULL")
                .add_column("fee_nanomina BIGINT NOT NULL")
                .add_column("is_canonical BOOLEAN NOT NULL")
                .distinct_columns(&["height", "state_hash", "timestamp", "prover", "fee_nanomina"])
                .build(root_node)
                .await
                .expect("Failed to build snarks_log and snarks view");

            Self {
                id: "SnarkSummaryPersistenceActor".to_string(),
                shared_publisher,
                db_logger: Arc::new(Mutex::new(logger)),
                database_inserts: AtomicUsize::new(0),
            }
        } else {
            panic!("Unable to establish connection to database")
        }
    }

    async fn log(&self, summary: &SnarkCanonicitySummaryPayload) -> Result<u64, &'static str> {
        let logger = self.db_logger.lock().await;
        match logger
            .insert(&[
                &(summary.height as i64),
                &summary.state_hash,
                &(summary.timestamp as i64),
                &summary.prover,
                &(summary.fee_nanomina as i64),
                &summary.canonical,
            ])
            .await
        {
            Err(e) => {
                let msg = e.to_string();
                println!("{}", msg);
                Err("unable to upsert into snark_work_summary table")
            }
            Ok(affected_rows) => Ok(affected_rows),
        }
    }
}

#[async_trait]
impl Actor for SnarkSummaryPersistenceActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.database_inserts
    }
    async fn handle_event(&self, event: Event) {
        if event.event_type == EventType::SnarkCanonicitySummary {
            let event_payload: SnarkCanonicitySummaryPayload = sonic_rs::from_str(&event.payload).unwrap();
            match self.log(&event_payload).await {
                Ok(affected_rows) => {
                    assert_eq!(affected_rows, 1);
                    self.shared_publisher.incr_database_insert();
                    self.publish(Event {
                        event_type: EventType::ActorHeight,
                        payload: sonic_rs::to_string(&ActorHeightPayload {
                            actor: self.id(),
                            height: event_payload.height,
                        })
                        .unwrap(),
                    });
                }
                Err(e) => {
                    panic!("{}", e);
                }
            }
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[cfg(test)]
mod snark_summary_persistence_actor_tests {
    use super::*;
    use crate::stream::payloads::{ActorHeightPayload, SnarkCanonicitySummaryPayload};
    use std::sync::Arc;
    use tokio::time::timeout;

    async fn setup_actor() -> (SnarkSummaryPersistenceActor, tokio::sync::broadcast::Receiver<Event>) {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = SnarkSummaryPersistenceActor::new(Arc::clone(&shared_publisher), &None).await;
        let receiver = shared_publisher.subscribe();
        (actor, receiver)
    }

    #[tokio::test]
    async fn test_snark_summary_persistence_actor_logs_summary() {
        let (actor, mut receiver) = setup_actor().await;

        let snark_summary = SnarkCanonicitySummaryPayload {
            height: 10,
            state_hash: "test_hash".to_string(),
            timestamp: 123456,
            prover: "test_prover".to_string(),
            fee_nanomina: 250000000,
            canonical: true,
        };

        let event = Event {
            event_type: EventType::SnarkCanonicitySummary,
            payload: sonic_rs::to_string(&snark_summary).unwrap(),
        };

        // Handle the event
        actor.handle_event(event).await;

        // Verify the ActorHeight event is published
        if let Ok(event) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            let published_event: ActorHeightPayload = sonic_rs::from_str(&event.unwrap().payload).unwrap();
            assert_eq!(published_event.actor, actor.id());
            assert_eq!(published_event.height, snark_summary.height);
        } else {
            panic!("Expected ActorHeight event was not published.");
        }
    }

    #[tokio::test]
    async fn test_snark_summary_persistence_actor_logs_to_database() {
        let (actor, _) = setup_actor().await;

        let snark_summary = SnarkCanonicitySummaryPayload {
            height: 15,
            state_hash: "test_hash_2".to_string(),
            timestamp: 789012,
            prover: "test_prover_2".to_string(),
            fee_nanomina: 500000000,
            canonical: false,
        };

        // Log the snark summary
        let result = actor.log(&snark_summary).await;

        // Verify successful database insertion
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_snark_summary_persistence_actor_handles_multiple_events() {
        let (actor, mut receiver) = setup_actor().await;

        let summaries = vec![
            SnarkCanonicitySummaryPayload {
                height: 20,
                state_hash: "hash_1".to_string(),
                timestamp: 111111,
                prover: "prover_1".to_string(),
                fee_nanomina: 1000000000,
                canonical: true,
            },
            SnarkCanonicitySummaryPayload {
                height: 21,
                state_hash: "hash_2".to_string(),
                timestamp: 222222,
                prover: "prover_2".to_string(),
                fee_nanomina: 2000000000,
                canonical: false,
            },
        ];

        for summary in &summaries {
            let event = Event {
                event_type: EventType::SnarkCanonicitySummary,
                payload: sonic_rs::to_string(&summary).unwrap(),
            };
            actor.handle_event(event).await;
        }

        // Verify ActorHeight events for both summaries
        for summary in summaries {
            if let Ok(event) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
                let published_event: ActorHeightPayload = sonic_rs::from_str(&event.unwrap().payload).unwrap();
                assert_eq!(published_event.actor, actor.id());
                assert_eq!(published_event.height, summary.height);
            } else {
                panic!("Expected ActorHeight event was not published.");
            }
        }
    }
}
