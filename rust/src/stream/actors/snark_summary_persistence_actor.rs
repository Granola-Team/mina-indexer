use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{constants::POSTGRES_CONNECTION_STRING, stream::payloads::*};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};
use tokio_postgres::{Client, NoTls};

pub struct SnarkSummaryPersistenceActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
    pub client: Client,
}

impl SnarkSummaryPersistenceActor {
    pub async fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        if let Ok((client, connection)) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls).await {
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {}", e);
                }
            });
            if let Err(e) = client
                .execute(
                    "CREATE TABLE IF NOT EXISTS snark_work_summary (
                        height BIGINT NOT NULL,
                        state_hash TEXT NOT NULL,
                        timestamp BIGINT NOT NULL,
                        prover TEXT NOT NULL,
                        fee DOUBLE PRECISION NOT NULL
                    );",
                    &[],
                )
                .await
            {
                println!("Unable to create snark_work_summary table {:?}", e);
            }
            Self {
                id: "SnarkSummaryPersistenceActor".to_string(),
                shared_publisher,
                client,
                events_published: AtomicUsize::new(0),
            }
        } else {
            panic!("Unable to establish connection to database")
        }
    }

    async fn db_upsert(&self, summary: &SnarkCanonicitySummaryPayload) -> Result<u64, &'static str> {
        let upsert_query = r#"
            INSERT INTO snark_work_summary (
                height,
                state_hash,
                timestamp,
                prover,
                fee,
                is_canonical
            ) VALUES
                ($1, $2, $3, $4, $5)
            ON CONFLICT (height, state_hash) DO UPDATE
            SET
                timestamp = EXCLUDED.timestamp,
                prover = EXCLUDED.prover,
                fee = EXCLUDED.fee,
                is_canonical = EXCLUDED.is_canonical;

        "#;

        match self
            .client
            .execute(
                upsert_query,
                &[
                    &(summary.height as i64),
                    &summary.state_hash,
                    &(summary.timestamp as i64),
                    &summary.prover,
                    &{ summary.fee },
                    &summary.canonical,
                ],
            )
            .await
        {
            Err(_) => Err("Unable to upsert into snark_work_summary table"),
            Ok(affected_rows) => Ok(affected_rows),
        }
    }
}

#[async_trait]
impl Actor for SnarkSummaryPersistenceActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn events_published(&self) -> &AtomicUsize {
        &self.events_published
    }
    async fn handle_event(&self, event: Event) {
        if event.event_type == EventType::SnarkCanonicitySummary {
            let event_payload: SnarkCanonicitySummaryPayload = sonic_rs::from_str(&event.payload).unwrap();
            if let Err(e) = self.db_upsert(&event_payload).await {
                panic!("{:?}", e);
            }
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}
