use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    constants::POSTGRES_CONNECTION_STRING,
    event_sourcing::{db_logger::DbLogger, payloads::*},
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

    async fn log_batch(&self, payload: &BatchSnarkCanonicityPayload) -> Result<(), &'static str> {
        let mut values = Vec::new();

        // Pre-allocate values for all rows
        for command in &payload.snarks {
            values.push((
                payload.height as i64,
                payload.state_hash.to_string(),
                payload.timestamp as i64,
                command.prover.to_string(),
                command.fee_nanomina as i64,
                payload.canonical,
            ));
        }

        // Create the rows referencing pre-allocated values
        let rows: Vec<Vec<&(dyn tokio_postgres::types::ToSql + Sync)>> = values
            .iter()
            .map(|(height, state_hash, timestamp, prover, fee_nanomina, canonical)| {
                vec![
                    height as &(dyn tokio_postgres::types::ToSql + Sync),
                    state_hash as &(dyn tokio_postgres::types::ToSql + Sync),
                    timestamp as &(dyn tokio_postgres::types::ToSql + Sync),
                    prover as &(dyn tokio_postgres::types::ToSql + Sync),
                    fee_nanomina as &(dyn tokio_postgres::types::ToSql + Sync),
                    canonical as &(dyn tokio_postgres::types::ToSql + Sync),
                ]
            })
            .collect();

        // Perform the bulk insert
        let logger = self.db_logger.lock().await;
        logger
            .bulk_insert(&rows)
            .await
            .map_err(|_| "Unable to bulk insert into canonical_user_command_log table")?;

        Ok(())
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
        if event.event_type == EventType::BulkSnarkCanonicity {
            let event_payload: BatchSnarkCanonicityPayload = sonic_rs::from_str(&event.payload).unwrap();
            match self.log_batch(&event_payload).await {
                Ok(_) => {
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
    use crate::event_sourcing::payloads::ActorHeightPayload;
    use std::sync::Arc;
    use tokio::time::timeout;

    async fn setup_actor() -> (SnarkSummaryPersistenceActor, tokio::sync::broadcast::Receiver<Event>) {
        let shared_publisher = Arc::new(SharedPublisher::new(100));

        let receiver = shared_publisher.subscribe();

        let actor = SnarkSummaryPersistenceActor::new(Arc::clone(&shared_publisher), &None).await;
        (actor, receiver)
    }

    #[tokio::test]
    async fn test_snark_summary_persistence_actor_logs_bulk_summary() {
        let (actor, mut receiver) = setup_actor().await;

        let bulk_snark_summary = BatchSnarkCanonicityPayload {
            height: 10,
            state_hash: "test_hash".to_string(),
            timestamp: 123456,
            canonical: true,
            snarks: vec![
                Snark {
                    prover: "test_prover_1".to_string(),
                    fee_nanomina: 250000000,
                },
                Snark {
                    prover: "test_prover_2".to_string(),
                    fee_nanomina: 300000000,
                },
            ],
        };

        let event = Event {
            event_type: EventType::BulkSnarkCanonicity,
            payload: sonic_rs::to_string(&bulk_snark_summary).unwrap(),
        };

        // Handle the event
        actor.handle_event(event).await;

        // Verify the ActorHeight event is published
        if let Ok(event) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            let published_event: ActorHeightPayload = sonic_rs::from_str(&event.unwrap().payload).unwrap();
            assert_eq!(published_event.actor, actor.id());
            assert_eq!(published_event.height, bulk_snark_summary.height);
        } else {
            panic!("Expected ActorHeight event was not published.");
        }

        // Validate the data was logged into the database
        let query = "SELECT * FROM snarks WHERE height = $1 AND state_hash = $2";
        let logger = actor.db_logger.lock().await;
        let rows = logger
            .get_client()
            .query(query, &[&(bulk_snark_summary.height as i64), &bulk_snark_summary.state_hash])
            .await
            .unwrap();

        // Ensure all snarks are present
        assert_eq!(rows.len(), bulk_snark_summary.snarks.len());

        // Validate the details of each snark
        for (row, snark) in rows.iter().zip(&bulk_snark_summary.snarks) {
            assert_eq!(row.get::<_, String>("prover"), snark.prover);
            assert_eq!(row.get::<_, i64>("fee_nanomina"), snark.fee_nanomina as i64);
            assert_eq!(row.get::<_, bool>("is_canonical"), bulk_snark_summary.canonical);
        }
    }

    #[tokio::test]
    async fn test_snark_summary_persistence_actor_handles_two_batch_events() {
        let (actors, mut receiver) = setup_actor().await;

        // Create the first batch payload
        let batch_summary1 = BatchSnarkCanonicityPayload {
            height: 20,
            state_hash: "hash_1".to_string(),
            timestamp: 111111,
            canonical: true,
            snarks: vec![Snark {
                prover: "prover_1".to_string(),
                fee_nanomina: 1000000000,
            }],
        };

        // Create the second batch payload
        let batch_summary2 = BatchSnarkCanonicityPayload {
            height: 21,
            state_hash: "hash_2".to_string(),
            timestamp: 222222,
            canonical: false,
            snarks: vec![Snark {
                prover: "prover_2".to_string(),
                fee_nanomina: 2000000000,
            }],
        };

        // Send the first batch event
        let event1 = Event {
            event_type: EventType::BulkSnarkCanonicity,
            payload: sonic_rs::to_string(&batch_summary1).unwrap(),
        };
        actors.handle_event(event1).await;

        // Send the second batch event
        let event2 = Event {
            event_type: EventType::BulkSnarkCanonicity,
            payload: sonic_rs::to_string(&batch_summary2).unwrap(),
        };
        actors.handle_event(event2).await;

        // Verify ActorHeight events for the first batch
        if let Ok(event) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            let published_event: ActorHeightPayload = sonic_rs::from_str(&event.unwrap().payload).unwrap();
            assert_eq!(published_event.actor, actors.id());
            assert_eq!(published_event.height, batch_summary1.height);
        } else {
            panic!("Expected ActorHeight event for the first batch was not published.");
        }

        // Verify ActorHeight events for the second batch
        if let Ok(event) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            let published_event: ActorHeightPayload = sonic_rs::from_str(&event.unwrap().payload).unwrap();
            assert_eq!(published_event.actor, actors.id());
            assert_eq!(published_event.height, batch_summary2.height);
        } else {
            panic!("Expected ActorHeight event for the second batch was not published.");
        }
    }
}
