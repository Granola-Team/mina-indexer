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

pub struct CanonicalUserCommandPersistenceActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub db_logger: Arc<Mutex<DbLogger>>,
    pub database_inserts: AtomicUsize,
    pub modulo_3: u64,
}

impl CanonicalUserCommandPersistenceActor {
    pub async fn new(shared_publisher: Arc<SharedPublisher>, root_node: &Option<(u64, String)>, modulo_3: u64) -> Self {
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Unable to connect to database");
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });
        let logger = DbLogger::builder(client)
            .name("user_commands")
            .add_column("height BIGINT")
            .add_column("txn_hash TEXT")
            .add_column("state_hash TEXT")
            .add_column("timestamp BIGINT")
            .add_column("txn_type TEXT")
            .add_column("status TEXT")
            .add_column("sender TEXT")
            .add_column("receiver TEXT")
            .add_column("nonce BIGINT")
            .add_column("fee_nanomina BIGINT")
            .add_column("fee_payer TEXT")
            .add_column("amount_nanomina BIGINT")
            .add_column("canonical BOOLEAN")
            .distinct_columns(&["height", "txn_hash", "state_hash"])
            .build(root_node)
            .await
            .expect("Failed to build user_commands_log and user_commands view");

        Self {
            id: "CanonicalUserCommandPersistenceActor".to_string(),
            shared_publisher,
            modulo_3,
            db_logger: Arc::new(Mutex::new(logger)),
            database_inserts: AtomicUsize::new(0),
        }
    }

    async fn log(&self, payload: &CanonicalUserCommandLogPayload) -> Result<(), &'static str> {
        let logger = self.db_logger.lock().await;
        logger
            .insert(&[
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
            ])
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
            if log.height % 3 != self.modulo_3 {
                return;
            }
            self.log(&log).await.unwrap();
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

    async fn setup_actor() -> (
        CanonicalUserCommandPersistenceActor,
        CanonicalUserCommandPersistenceActor,
        CanonicalUserCommandPersistenceActor,
        Arc<SharedPublisher>,
        broadcast::Receiver<Event>,
    ) {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let receiver = shared_publisher.subscribe();

        let actor_m0 = CanonicalUserCommandPersistenceActor::new(Arc::clone(&shared_publisher), &None, 0).await;
        let actor_m1 = CanonicalUserCommandPersistenceActor::new(Arc::clone(&shared_publisher), &None, 1).await;
        let actor_m2 = CanonicalUserCommandPersistenceActor::new(Arc::clone(&shared_publisher), &None, 2).await;

        (actor_m0, actor_m1, actor_m2, shared_publisher, receiver)
    }

    #[tokio::test]
    async fn test_insert_canonical_user_command_log() {
        let (_, actor_m1, _, _shared_publisher, _receiver) = setup_actor().await;

        let payload = CanonicalUserCommandLogPayload {
            height: 100,
            global_slot: 0,
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

        actor_m1.log(&payload).await.unwrap();

        let query = "SELECT * FROM user_commands_log WHERE height = $1 AND state_hash = $2 AND timestamp = $3";
        let db_logger = actor_m1.db_logger.lock().await;
        let row = db_logger
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
        let (_, _, actor_m2, _shared_publisher, _receiver) = setup_actor().await;

        let payload = CanonicalUserCommandLogPayload {
            height: 200,
            global_slot: 0,
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

        actor_m2.handle_event(event).await;

        let query = "SELECT * FROM user_commands_log WHERE height = $1 AND state_hash = $2 AND timestamp = $3";
        let db_logger = actor_m2.db_logger.lock().await;
        let row = db_logger
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
        let (_, _, actor_m2, _shared_publisher, _receiver) = setup_actor().await;

        // Insert multiple entries for the same (height, state_hash) with different statuses
        let payload1 = CanonicalUserCommandLogPayload {
            height: 1,
            global_slot: 0,
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
            global_slot: 0,
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
        actor_m2.log(&payload1).await.unwrap();
        actor_m2.log(&payload2).await.unwrap();

        // Query the database to ensure the correct row is returned
        let query = "SELECT * FROM user_commands WHERE height = $1 AND state_hash = $2 and txn_hash = $3 ORDER BY timestamp DESC";
        let db_logger = actor_m2.db_logger.lock().await;
        let rows = db_logger
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
        let (_, _, actor_m2, _shared_publisher, mut receiver) = setup_actor().await;

        // Create a payload for a canonical user command
        let payload = CanonicalUserCommandLogPayload {
            height: 200,
            global_slot: 0,
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

        actor_m2.handle_event(event).await;

        // Listen for the `ActorHeight` event
        let received_event = receiver.recv().await.unwrap();

        // Verify the event type is `ActorHeight`
        assert_eq!(received_event.event_type, EventType::ActorHeight);

        // Deserialize the payload
        let actor_height_payload: ActorHeightPayload = sonic_rs::from_str(&received_event.payload).unwrap();

        // Validate the `ActorHeightPayload` content
        assert_eq!(actor_height_payload.actor, actor_m2.id());
        assert_eq!(actor_height_payload.height, payload.height);
    }
}
