use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    constants::POSTGRES_CONNECTION_STRING,
    stream::{models::Height, payloads::*},
};
use anyhow::Result;
use async_trait::async_trait;
use futures::lock::Mutex;
use std::{
    collections::{HashMap, VecDeque},
    sync::{atomic::AtomicUsize, Arc},
};
use tokio_postgres::{Client, NoTls};

pub struct SnarkSummaryPersistenceActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
    pub block_canonicity_queue: Arc<Mutex<VecDeque<BlockCanonicityUpdatePayload>>>,
    pub snark_map: Arc<Mutex<HashMap<Height, Vec<SnarkWorkSummaryPayload>>>>,
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
                    "CREATE TABLE snark_work_summary_table (
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
                println!("Unable to create snark_work_summary_table table {:?}", e);
            }
            Self {
                id: "SnarkSummaryPersistenceActor".to_string(),
                shared_publisher,
                client,
                events_published: AtomicUsize::new(0),
                block_canonicity_queue: Arc::new(Mutex::new(VecDeque::new())),
                snark_map: Arc::new(Mutex::new(HashMap::new())),
            }
        } else {
            panic!("Unable to establish connection to database")
        }
    }

    async fn db_upsert(&self, summary: &SnarkWorkSummaryPayload, canonical: bool) -> Result<u64, &'static str> {
        let upsert_query = r#"
            INSERT INTO snark_work_summary_table (
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
                    &canonical,
                ],
            )
            .await
        {
            Err(_) => Err("Unable to upsert into snark_work_summary_table table"),
            Ok(affected_rows) => Ok(affected_rows),
        }
    }

    async fn upsert_snark_summary(&self) -> Result<(), &'static str> {
        let mut queue = self.block_canonicity_queue.lock().await;
        // Continue looping until the queue is empty
        while let Some(update) = queue.pop_front() {
            let snarks = self.snark_map.lock().await;

            if let Some(entries) = snarks.get(&Height(update.height)) {
                for entry in entries {
                    match self.db_upsert(entry, update.canonical).await {
                        Ok(affected_rows) => {
                            assert_eq!(affected_rows, 1, "Snark summary update did not update a single row");
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                }
            } else {
                queue.push_front(update);
                drop(queue);
                break;
            }
        }

        Ok(())
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
        match event.event_type {
            EventType::BlockCanonicityUpdate => {
                let mut queue = self.block_canonicity_queue.lock().await;
                queue.push_back(sonic_rs::from_str(&event.payload).unwrap());
                drop(queue);
            }
            EventType::SnarkWorkSummary => {
                let event_payload: SnarkWorkSummaryPayload = sonic_rs::from_str(&event.payload).unwrap();
                let mut snarks = self.snark_map.lock().await;
                snarks.entry(Height(event_payload.height)).or_insert_with(Vec::new).push(event_payload);
                drop(snarks);
            }
            _ => return,
        }
        if let Err(e) = self.upsert_snark_summary().await {
            panic!("{:?}", e);
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}
