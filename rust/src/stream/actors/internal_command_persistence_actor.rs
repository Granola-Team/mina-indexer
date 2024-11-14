use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{constants::POSTGRES_CONNECTION_STRING, stream::payloads::InternalCommandCanonicityPayload};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};
use tokio_postgres::{Client, NoTls};

pub struct InternalCommandPersistenceActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub database_inserts: AtomicUsize,
    pub client: Client,
}

impl InternalCommandPersistenceActor {
    pub async fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        if let Ok((client, connection)) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls).await {
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {}", e);
                }
            });
            if let Err(e) = client
                .execute(
                    "CREATE TABLE IF NOT EXISTS internal_commands (
                        internal_command_type TEXT NOT NULL,
                        height BIGINT NOT NULL,
                        state_hash TEXT NOT NULL,
                        timestamp BIGINT NOT NULL,
                        amount_nanomina BIGINT NOT NULL,
                        recipient TEXT NOT NULL,
                        is_canonical BOOLEAN NOT NULL,
                        CONSTRAINT unique_internal_command UNIQUE (
                            internal_command_type,
                            height,
                            state_hash,
                            timestamp,
                            amount_nanomina,
                            recipient
                        )
                    );",
                    &[],
                )
                .await
            {
                println!("Unable to create internal_commands table {:?}", e);
            }
            Self {
                id: "InternalCommandPersistenceActor".to_string(),
                shared_publisher,
                client,
                database_inserts: AtomicUsize::new(0),
            }
        } else {
            panic!("Unable to establish connection to database")
        }
    }

    async fn db_upsert(&self, payload: &InternalCommandCanonicityPayload) -> Result<u64, &'static str> {
        let upsert_query = r#"
            INSERT INTO internal_commands (
                internal_command_type,
                height,
                state_hash,
                timestamp,
                amount_nanomina,
                recipient,
                is_canonical
            ) VALUES
                ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT ON CONSTRAINT unique_internal_command
            DO UPDATE SET
                is_canonical = EXCLUDED.is_canonical;
            "#;

        match self
            .client
            .execute(
                upsert_query,
                &[
                    &payload.internal_command_type.to_string(),
                    &(payload.height as i64),
                    &payload.state_hash,
                    &(payload.timestamp as i64),
                    &(payload.amount_nanomina as i64),
                    &payload.recipient,
                    &payload.canonical,
                ],
            )
            .await
        {
            Err(e) => {
                let msg = e.to_string();
                println!("{}", msg);
                Err("unable to upsert into internal_commands table")
            }
            Ok(affected_rows) => Ok(affected_rows),
        }
    }
}

#[async_trait]
impl Actor for InternalCommandPersistenceActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.database_inserts
    }

    async fn handle_event(&self, event: Event) {
        if event.event_type == EventType::InternalCommandCanonicityUpdate {
            let event_payload: InternalCommandCanonicityPayload = sonic_rs::from_str(&event.payload).unwrap();
            match self.db_upsert(&event_payload).await {
                Ok(affected_rows) => {
                    assert_eq!(affected_rows, 1);
                    self.shared_publisher.incr_database_insert();
                    self.actor_outputs().fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
