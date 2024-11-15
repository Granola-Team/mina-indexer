use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{constants::POSTGRES_CONNECTION_STRING, stream::payloads::UserCommandCanonicityPayload};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};
use tokio_postgres::{Client, NoTls};

pub struct UserCommandPersistenceActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub database_inserts: AtomicUsize,
    pub client: Client,
}

impl UserCommandPersistenceActor {
    pub async fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        if let Ok((client, connection)) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls).await {
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {}", e);
                }
            });
            if let Err(e) = client
                .execute(
                    "CREATE TABLE IF NOT EXISTS user_commands (
                        height BIGINT NOT NULL,
                        state_hash TEXT NOT NULL,
                        timestamp BIGINT NOT NULL,
                        txn_type TEXT NOT NULL,
                        status TEXT NOT NULL,
                        sender TEXT NOT NULL,
                        receiver TEXT NOT NULL,
                        nonce INTEGER NOT NULL,
                        fee_nanomina BIGINT NOT NULL,
                        amount_nanomina BIGINT NOT NULL,
                        is_canonical BOOLEAN NOT NULL,
                        CONSTRAINT unique_user_command UNIQUE (height, state_hash, nonce)
                    );",
                    &[],
                )
                .await
            {
                println!("Unable to create user_command table {:?}", e);
            }
            Self {
                id: "UserCommandPersistenceActor".to_string(),
                shared_publisher,
                client,
                database_inserts: AtomicUsize::new(0),
            }
        } else {
            panic!("Unable to establish connection to database")
        }
    }

    async fn db_upsert(&self, payload: &UserCommandCanonicityPayload) -> Result<u64, &'static str> {
        let upsert_query = r#"
            INSERT INTO user_commands (
                height,
                state_hash,
                timestamp,
                txn_type,
                status,
                sender,
                receiver,
                nonce,
                fee_nanomina,
                amount_nanomina,
                is_canonical
            ) VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT ON CONSTRAINT unique_user_command
            DO UPDATE SET
                state_hash = EXCLUDED.state_hash,
                timestamp = EXCLUDED.timestamp,
                txn_type = EXCLUDED.txn_type,
                status = EXCLUDED.status,
                sender = EXCLUDED.sender,
                receiver = EXCLUDED.receiver,
                fee_nanomina = EXCLUDED.fee_nanomina,
                amount_nanomina = EXCLUDED.amount_nanomina,
                is_canonical = EXCLUDED.is_canonical;
            "#;

        match self
            .client
            .execute(
                upsert_query,
                &[
                    &(payload.height as i64),
                    &payload.state_hash,
                    &(payload.timestamp as i64),
                    &payload.txn_type,
                    &payload.status.to_string(),
                    &payload.sender,
                    &payload.receiver,
                    &(payload.nonce as i32),
                    &(payload.fee_nanomina as i64),
                    &(payload.amount_nanomina as i64),
                    &payload.canonical,
                ],
            )
            .await
        {
            Err(e) => {
                let msg = e.to_string();
                println!("{}", msg);
                Err("unable to upsert into user_command table")
            }
            Ok(affected_rows) => Ok(affected_rows),
        }
    }
}

#[async_trait]
impl Actor for UserCommandPersistenceActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.database_inserts
    }
    async fn handle_event(&self, event: Event) {
        if event.event_type == EventType::UserCommandCanonicityUpdate {
            let event_payload: UserCommandCanonicityPayload = sonic_rs::from_str(&event.payload).unwrap();
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
