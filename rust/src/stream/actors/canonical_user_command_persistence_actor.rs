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

pub struct CanonicalUserCommandPersistenceActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub client: Client,
    pub database_inserts: AtomicUsize,
}

impl CanonicalUserCommandPersistenceActor {
    pub async fn new(shared_publisher: Arc<SharedPublisher>, preserve_existing_data: bool) -> Self {
        if let Ok((client, connection)) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls).await {
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {}", e);
                }
            });
            if !preserve_existing_data {
                if let Err(e) = client.execute("DROP TABLE IF EXISTS canonical_user_command_log CASCADE;", &[]).await {
                    println!("Unable to drop canonical_user_command_log table {:?}", e);
                }
            }
            if let Err(e) = client
                .execute(
                    "CREATE TABLE IF NOT EXISTS canonical_user_command_log (
                        height BIGINT,
                        txn_hash TEXT,
                        state_hash TEXT,
                        timestamp BIGINT,
                        txn_type TEXT,
                        status TEXT,
                        sender TEXT,
                        receiver TEXT,
                        nonce BIGINT,
                        fee_nanomina BIGINT,
                        fee_payer TEXT,
                        amount_nanomina BIGINT,
                        canonical BOOLEAN,
                        entry_id BIGSERIAL PRIMARY KEY
                    );",
                    &[],
                )
                .await
            {
                println!("Unable to create canonical_user_command_log table {:?}", e);
            }
            if let Err(e) = client
                .execute(
                    "CREATE OR REPLACE VIEW canonical_user_commands AS
                    SELECT DISTINCT ON (height, txn_hash, state_hash) *
                    FROM canonical_user_command_log
                    ORDER BY height, txn_hash, state_hash, entry_id DESC;",
                    &[],
                )
                .await
            {
                println!("Unable to create canonical_user_command_log table {:?}", e);
            }
            Self {
                id: "CanonicalUserCommandPersistenceActor".to_string(),
                shared_publisher,
                client,
                database_inserts: AtomicUsize::new(0),
            }
        } else {
            panic!("Unable to establish connection to database");
        }
    }

    async fn insert_canonical_user_command_log(&self, payload: &CanonicalUserCommandLogPayload) -> Result<(), &'static str> {
        let insert_query = r#"
            INSERT INTO canonical_user_command_log (
                height, txn_hash, state_hash, timestamp, txn_type, status, sender, receiver, nonce,
                fee_nanomina, fee_payer, amount_nanomina, canonical
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13
            )
        "#;

        self.client
            .execute(
                insert_query,
                &[
                    &(payload.height as i64),
                    &payload.txn_hash,
                    &payload.state_hash,
                    &(payload.timestamp as i64),
                    &format!("{:?}", payload.txn_type),
                    &format!("{:?}", payload.status),
                    &payload.sender,
                    &payload.receiver,
                    &(payload.nonce as i64),
                    &(payload.fee_nanomina as i64),
                    &payload.fee_payer,
                    &(payload.amount_nanomina as i64),
                    &payload.canonical,
                ],
            )
            .await
            .map_err(|_| "Unable to insert into canonical_user_command_log table")?;

        Ok(())
    }
}

#[async_trait]
impl Actor for CanonicalUserCommandPersistenceActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.database_inserts
    }

    async fn handle_event(&self, event: Event) {
        if event.event_type == EventType::CanonicalUserCommandLog {
            let log: CanonicalUserCommandLogPayload = sonic_rs::from_str(&event.payload).unwrap();
            self.insert_canonical_user_command_log(&log).await.unwrap();
            self.database_inserts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.publish(Event {
                event_type: EventType::ActorHeight,
                payload: sonic_rs::to_string(&ActorHeightPayload {
                    actor: self.id(),
                    height: log.height,
                })
                .unwrap(),
            });
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[cfg(test)]
mod canonical_user_command_log_persistence_tests {
    use super::*;
    use crate::stream::{
        events::{Event, EventType},
        mainnet_block_models::{CommandStatus, CommandType},
    };
    use std::sync::Arc;
    use tokio::sync::broadcast;

    async fn setup_actor() -> (CanonicalUserCommandPersistenceActor, Arc<SharedPublisher>, broadcast::Receiver<Event>) {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let receiver = shared_publisher.subscribe();

        let actor = CanonicalUserCommandPersistenceActor::new(Arc::clone(&shared_publisher), false).await;

        (actor, shared_publisher, receiver)
    }

    #[tokio::test]
    async fn test_insert_canonical_user_command_log() {
        let (actor, _shared_publisher, _receiver) = setup_actor().await;

        let payload = CanonicalUserCommandLogPayload {
            height: 100,
            txn_hash: "txn_hash_1".to_string(),
            state_hash: "state_hash_100".to_string(),
            timestamp: 1234567890,
            txn_type: CommandType::Payment,
            status: CommandStatus::Applied,
            sender: "sender_1".to_string(),
            receiver: "receiver_1".to_string(),
            nonce: 42,
            fee_nanomina: 1000,
            fee_payer: "fee_payer_1".to_string(),
            amount_nanomina: 5000,
            canonical: true,
            was_canonical: false,
        };

        actor.insert_canonical_user_command_log(&payload).await.unwrap();

        let query = "SELECT * FROM canonical_user_command_log WHERE height = $1 AND state_hash = $2 AND timestamp = $3";
        let row = actor
            .client
            .query_one(query, &[&(payload.height as i64), &payload.state_hash, &(payload.timestamp as i64)])
            .await
            .unwrap();

        assert_eq!(row.get::<_, i64>("height"), payload.height as i64);
        assert_eq!(row.get::<_, String>("state_hash"), payload.state_hash);
        assert_eq!(row.get::<_, i64>("timestamp"), payload.timestamp as i64);
        assert_eq!(row.get::<_, String>("sender"), payload.sender);
        assert_eq!(row.get::<_, String>("receiver"), payload.receiver);
        assert_eq!(row.get::<_, i64>("nonce"), payload.nonce as i64);
        assert_eq!(row.get::<_, i64>("fee_nanomina"), payload.fee_nanomina as i64);
        assert_eq!(row.get::<_, String>("fee_payer"), payload.fee_payer);
        assert_eq!(row.get::<_, i64>("amount_nanomina"), payload.amount_nanomina as i64);
        assert_eq!(row.get::<_, bool>("canonical"), payload.canonical);
    }

    #[tokio::test]
    async fn test_handle_event_canonical_user_command_log() {
        let (actor, _, _) = setup_actor().await;

        let payload = CanonicalUserCommandLogPayload {
            height: 200,
            txn_hash: "txn_hash_1".to_string(),
            state_hash: "state_hash_200".to_string(),
            timestamp: 987654321,
            txn_type: CommandType::Payment,
            status: CommandStatus::Failed,
            sender: "sender_2".to_string(),
            receiver: "receiver_2".to_string(),
            nonce: 99,
            fee_nanomina: 2000,
            fee_payer: "fee_payer_2".to_string(),
            amount_nanomina: 10000,
            canonical: false,
            was_canonical: true,
        };

        let event = Event {
            event_type: EventType::CanonicalUserCommandLog,
            payload: sonic_rs::to_string(&payload).unwrap(),
        };

        actor.handle_event(event).await;

        let query = "SELECT * FROM canonical_user_command_log WHERE height = $1 AND state_hash = $2 AND timestamp = $3";
        let row = actor
            .client
            .query_one(query, &[&(payload.height as i64), &payload.state_hash, &(payload.timestamp as i64)])
            .await
            .unwrap();

        assert_eq!(row.get::<_, i64>("height"), payload.height as i64);
        assert_eq!(row.get::<_, String>("state_hash"), payload.state_hash);
        assert_eq!(row.get::<_, i64>("timestamp"), payload.timestamp as i64);
        assert_eq!(row.get::<_, String>("sender"), payload.sender);
        assert_eq!(row.get::<_, String>("receiver"), payload.receiver);
        assert_eq!(row.get::<_, i64>("nonce"), payload.nonce as i64);
        assert_eq!(row.get::<_, i64>("fee_nanomina"), payload.fee_nanomina as i64);
        assert_eq!(row.get::<_, String>("fee_payer"), payload.fee_payer);
        assert_eq!(row.get::<_, i64>("amount_nanomina"), payload.amount_nanomina as i64);
        assert_eq!(row.get::<_, bool>("canonical"), payload.canonical);
    }

    #[tokio::test]
    async fn test_canonical_user_command_view() {
        // Set up the actor and database connection
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = CanonicalUserCommandPersistenceActor::new(Arc::clone(&shared_publisher), false).await;

        // Insert multiple entries for the same (height, state_hash) with different statuses
        let payload1 = CanonicalUserCommandLogPayload {
            height: 1,
            txn_hash: "txn_hash_1".to_string(),
            state_hash: "hash_1".to_string(),
            timestamp: 1234567890,
            txn_type: CommandType::Payment,
            status: CommandStatus::Applied,
            sender: "sender_1".to_string(),
            receiver: "receiver_1".to_string(),
            nonce: 1,
            fee_nanomina: 500,
            fee_payer: "payer_1".to_string(),
            amount_nanomina: 10000,
            canonical: false,
            was_canonical: false,
        };

        let payload2 = CanonicalUserCommandLogPayload {
            height: 1,
            txn_hash: "txn_hash_1".to_string(),
            state_hash: "hash_1".to_string(),
            timestamp: 1234567891, // Later timestamp
            txn_type: CommandType::Payment,
            status: CommandStatus::Applied,
            sender: "sender_1".to_string(),
            receiver: "receiver_1".to_string(),
            nonce: 1,
            fee_nanomina: 700,
            fee_payer: "payer_1".to_string(),
            amount_nanomina: 12000,
            canonical: true,
            was_canonical: true,
        };

        // Insert the payloads into the database
        actor.insert_canonical_user_command_log(&payload1).await.unwrap();
        actor.insert_canonical_user_command_log(&payload2).await.unwrap();

        // Query the database to ensure the correct row is returned
        let query = "SELECT * FROM canonical_user_commands WHERE height = $1 AND state_hash = $2 and txn_hash = $3 ORDER BY timestamp DESC";
        let rows = actor
            .client
            .query(query, &[&(payload1.height as i64), &payload1.state_hash, &payload1.txn_hash])
            .await
            .unwrap();

        // Ensure only the latest entry with the correct canonical state is returned
        assert_eq!(rows.len(), 1);

        let row = &rows[0];
        assert_eq!(row.get::<_, i64>("height"), payload2.height as i64);
        assert_eq!(row.get::<_, String>("state_hash"), payload2.state_hash);
        assert_eq!(row.get::<_, String>("txn_hash"), payload2.txn_hash);
        assert_eq!(row.get::<_, bool>("canonical"), payload2.canonical);
    }

    #[tokio::test]
    async fn test_actor_height_event_published() {
        let (actor, _, mut receiver) = setup_actor().await;

        // Create a payload for a canonical user command
        let payload = CanonicalUserCommandLogPayload {
            height: 200,
            txn_hash: "txn_hash_2".to_string(),
            state_hash: "state_hash_200".to_string(),
            timestamp: 987654321,
            txn_type: CommandType::Payment,
            status: CommandStatus::Applied,
            sender: "sender_2".to_string(),
            receiver: "receiver_2".to_string(),
            nonce: 43,
            fee_nanomina: 1500,
            fee_payer: "fee_payer_2".to_string(),
            amount_nanomina: 7500,
            canonical: true,
            was_canonical: false,
        };

        // Create and publish the event
        let event = Event {
            event_type: EventType::CanonicalUserCommandLog,
            payload: sonic_rs::to_string(&payload).unwrap(),
        };

        actor.handle_event(event).await;

        // Listen for the `ActorHeight` event
        let received_event = receiver.recv().await.unwrap();

        // Verify the event type is `ActorHeight`
        assert_eq!(received_event.event_type, EventType::ActorHeight);

        // Deserialize the payload
        let actor_height_payload: ActorHeightPayload = sonic_rs::from_str(&received_event.payload).unwrap();

        // Validate the `ActorHeightPayload` content
        assert_eq!(actor_height_payload.actor, actor.id());
        assert_eq!(actor_height_payload.height, payload.height);
    }
}