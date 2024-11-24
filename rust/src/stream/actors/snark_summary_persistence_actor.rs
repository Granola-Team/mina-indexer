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
                .add_column("fee DOUBLE PRECISION NOT NULL")
                .add_column("is_canonical BOOLEAN NOT NULL")
                .distinct_columns(&["height", "state_hash", "timestamp", "prover", "fee"])
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
