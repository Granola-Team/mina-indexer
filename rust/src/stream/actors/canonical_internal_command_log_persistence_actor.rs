use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{constants::POSTGRES_CONNECTION_STRING, stream::payloads::CanonicalInternalCommandLogPayload};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};
use tokio_postgres::{Client, NoTls};

pub struct CanonicalInternalCommandLogPersistenceActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub database_inserts: AtomicUsize,
    pub client: Client,
}

impl CanonicalInternalCommandLogPersistenceActor {
    pub async fn new(shared_publisher: Arc<SharedPublisher>, preserve_existing_data: bool) -> Self {
        if let Ok((client, connection)) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls).await {
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {}", e);
                }
            });
            if !preserve_existing_data {
                if let Err(e) = client.execute("DROP TABLE IF EXISTS canonical_internal_commands_log CASCADE;", &[]).await {
                    println!("Unable to drop internal_commands table {:?}", e);
                }
            }
            if let Err(e) = client
                .execute(
                    "CREATE TABLE IF NOT EXISTS canonical_internal_commands_log (
                        internal_command_type TEXT NOT NULL,
                        height BIGINT NOT NULL,
                        state_hash TEXT NOT NULL,
                        timestamp BIGINT NOT NULL,
                        amount_nanomina BIGINT NOT NULL,
                        recipient TEXT NOT NULL,
                        is_canonical BOOLEAN NOT NULL,
                        entry_id BIGSERIAL PRIMARY KEY
                    );",
                    &[],
                )
                .await
            {
                println!("Unable to create canonical_internal_commands_log table {:?}", e);
            }

            if let Err(e) = client
                .execute(
                    "CREATE OR REPLACE VIEW canonical_internal_commands AS
                    SELECT DISTINCT ON (height, internal_command_type, state_hash, recipient, amount_nanomina) *
                    FROM canonical_internal_commands_log
                    ORDER BY height, internal_command_type, state_hash, recipient, amount_nanomina, entry_id DESC;",
                    &[],
                )
                .await
            {
                println!("Unable to create canonical_internal_commands_log table {:?}", e);
            }
            Self {
                id: "CanonicalInternalCommandLogPersistenceActor".to_string(),
                shared_publisher,
                client,
                database_inserts: AtomicUsize::new(0),
            }
        } else {
            panic!("Unable to establish connection to database")
        }
    }

    async fn db_upsert(&self, payload: &CanonicalInternalCommandLogPayload) -> Result<u64, &'static str> {
        let upsert_query = r#"
            INSERT INTO canonical_internal_commands_log (
                internal_command_type,
                height,
                state_hash,
                timestamp,
                amount_nanomina,
                recipient,
                is_canonical
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
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
                Err("unable to upsert into canonical_internal_commands_log table")
            }
            Ok(affected_rows) => Ok(affected_rows),
        }
    }
}

#[async_trait]
impl Actor for CanonicalInternalCommandLogPersistenceActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.database_inserts
    }

    async fn handle_event(&self, event: Event) {
        if event.event_type == EventType::CanonicalInternalCommandLog {
            let event_payload: CanonicalInternalCommandLogPayload = sonic_rs::from_str(&event.payload).unwrap();
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

#[cfg(test)]
mod canonical_internal_command_log_tests {
    use super::*;
    use crate::stream::{
        events::{Event, EventType},
        payloads::InternalCommandType,
    };
    use std::sync::Arc;
    use tokio::sync::broadcast;

    async fn setup_actor() -> (CanonicalInternalCommandLogPersistenceActor, Arc<SharedPublisher>, broadcast::Receiver<Event>) {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let receiver = shared_publisher.subscribe();

        let actor = CanonicalInternalCommandLogPersistenceActor::new(Arc::clone(&shared_publisher), false).await;

        (actor, shared_publisher, receiver)
    }

    #[tokio::test]
    async fn test_insert_canonical_internal_command_log() {
        let (actor, _shared_publisher, _receiver) = setup_actor().await;

        let payload = CanonicalInternalCommandLogPayload {
            internal_command_type: InternalCommandType::FeeTransfer,
            height: 100,
            state_hash: "state_hash_100".to_string(),
            timestamp: 1234567890,
            amount_nanomina: 500,
            recipient: "recipient_1".to_string(),
            canonical: true,
            source: None,
            was_canonical: false,
        };

        let affected_rows = actor.db_upsert(&payload).await.unwrap();

        // Validate that exactly one row was affected
        assert_eq!(affected_rows, 1);

        // Validate the data was correctly inserted into the table
        let query = "SELECT * FROM canonical_internal_commands_log WHERE height = $1 AND state_hash = $2 AND recipient = $3";
        let row = actor
            .client
            .query_one(query, &[&(payload.height as i64), &payload.state_hash, &payload.recipient])
            .await
            .unwrap();

        assert_eq!(row.get::<_, String>("internal_command_type"), payload.internal_command_type.to_string());
        assert_eq!(row.get::<_, i64>("height"), payload.height as i64);
        assert_eq!(row.get::<_, String>("state_hash"), payload.state_hash);
        assert_eq!(row.get::<_, i64>("timestamp"), payload.timestamp as i64);
        assert_eq!(row.get::<_, i64>("amount_nanomina"), payload.amount_nanomina as i64);
        assert_eq!(row.get::<_, String>("recipient"), payload.recipient);
        assert_eq!(row.get::<_, bool>("is_canonical"), payload.canonical);
    }

    #[tokio::test]
    async fn test_handle_event_canonical_internal_command_log() {
        let (actor, _, _) = setup_actor().await;

        let payload = CanonicalInternalCommandLogPayload {
            internal_command_type: InternalCommandType::Coinbase,
            height: 200,
            state_hash: "state_hash_200".to_string(),
            timestamp: 987654321,
            amount_nanomina: 1000,
            recipient: "recipient_2".to_string(),
            canonical: false,
            was_canonical: false,
            source: None,
        };

        let event = Event {
            event_type: EventType::CanonicalInternalCommandLog,
            payload: sonic_rs::to_string(&payload).unwrap(),
        };

        actor.handle_event(event).await;

        // Validate the data was correctly inserted into the table
        let query = "SELECT * FROM canonical_internal_commands_log WHERE height = $1 AND state_hash = $2 AND recipient = $3";
        let row = actor
            .client
            .query_one(query, &[&(payload.height as i64), &payload.state_hash, &payload.recipient])
            .await
            .unwrap();

        assert_eq!(row.get::<_, String>("internal_command_type"), payload.internal_command_type.to_string());
        assert_eq!(row.get::<_, i64>("height"), payload.height as i64);
        assert_eq!(row.get::<_, String>("state_hash"), payload.state_hash);
        assert_eq!(row.get::<_, i64>("timestamp"), payload.timestamp as i64);
        assert_eq!(row.get::<_, i64>("amount_nanomina"), payload.amount_nanomina as i64);
        assert_eq!(row.get::<_, String>("recipient"), payload.recipient);
        assert_eq!(row.get::<_, bool>("is_canonical"), payload.canonical);
    }

    #[tokio::test]
    async fn test_canonical_internal_commands_view() {
        // Set up the actor and database connection
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = CanonicalInternalCommandLogPersistenceActor::new(Arc::clone(&shared_publisher), false).await;

        // Insert multiple entries for the same (height, state_hash, recipient, amount) with different timestamps and canonicalities
        let payload1 = CanonicalInternalCommandLogPayload {
            internal_command_type: InternalCommandType::FeeTransfer,
            height: 1,
            state_hash: "hash_1".to_string(),
            timestamp: 1234567890,
            amount_nanomina: 1000,
            recipient: "recipient_1".to_string(),
            canonical: false,
            was_canonical: false,
            source: None,
        };

        let payload2 = CanonicalInternalCommandLogPayload {
            internal_command_type: InternalCommandType::FeeTransfer,
            height: 1,
            state_hash: "hash_1".to_string(),
            timestamp: 1234567891, // Later timestamp
            amount_nanomina: 1000,
            recipient: "recipient_1".to_string(),
            canonical: true,
            was_canonical: false,
            source: None,
        };

        actor.db_upsert(&payload1).await.unwrap();
        actor.db_upsert(&payload2).await.unwrap();

        // Query the canonical_internal_commands view
        let query = "SELECT * FROM canonical_internal_commands WHERE height = $1 AND state_hash = $2 AND recipient = $3";
        let row = actor
            .client
            .query_one(query, &[&(payload1.height as i64), &payload1.state_hash, &payload1.recipient])
            .await
            .unwrap();

        // Validate that the row returned matches the payload with the highest timestamp
        assert_eq!(row.get::<_, String>("internal_command_type"), payload2.internal_command_type.to_string());
        assert_eq!(row.get::<_, i64>("height"), payload2.height as i64);
        assert_eq!(row.get::<_, String>("state_hash"), payload2.state_hash);
        assert_eq!(row.get::<_, i64>("timestamp"), payload2.timestamp as i64);
        assert_eq!(row.get::<_, i64>("amount_nanomina"), payload2.amount_nanomina as i64);
        assert_eq!(row.get::<_, String>("recipient"), payload2.recipient);
        assert_eq!(row.get::<_, bool>("is_canonical"), payload2.canonical);
    }
}
