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
use log::error;
use std::sync::{atomic::AtomicUsize, Arc};
use tokio_postgres::NoTls;

pub struct CanonicalUserCommandPersistenceActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub db_logger: Arc<Mutex<DbLogger>>,
    pub database_inserts: AtomicUsize,
}

impl CanonicalUserCommandPersistenceActor {
    pub async fn new(shared_publisher: Arc<SharedPublisher>, root_node: &Option<(u64, String)>) -> Self {
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Unable to connect to database");
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("connection error: {}", e);
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
            db_logger: Arc::new(Mutex::new(logger)),
            database_inserts: AtomicUsize::new(0),
        }
    }

    async fn log_batch(&self, payload: &BatchCanonicalUserCommandLogPayload) -> Result<(), &'static str> {
        let mut values = Vec::new();

        // Pre-allocate values for all rows
        for command in &payload.commands {
            values.push((
                payload.height as i64,
                command.txn_hash(),
                &payload.state_hash,
                payload.timestamp as i64,
                format!("{:?}", command.txn_type),
                format!("{:?}", command.status),
                &command.sender,
                &command.receiver,
                command.nonce as i64,
                command.fee_nanomina as i64,
                &command.fee_payer,
                command.amount_nanomina as i64,
                payload.canonical,
            ));
        }

        // Create the rows referencing pre-allocated values
        let rows: Vec<Vec<&(dyn tokio_postgres::types::ToSql + Sync)>> = values
            .iter()
            .map(
                |(height, txn_hash, state_hash, timestamp, txn_type, status, sender, receiver, nonce, fee_nanomina, fee_payer, amount_nanomina, canonical)| {
                    vec![
                        height as &(dyn tokio_postgres::types::ToSql + Sync),
                        txn_hash as &(dyn tokio_postgres::types::ToSql + Sync),
                        state_hash as &(dyn tokio_postgres::types::ToSql + Sync),
                        timestamp as &(dyn tokio_postgres::types::ToSql + Sync),
                        txn_type as &(dyn tokio_postgres::types::ToSql + Sync),
                        status as &(dyn tokio_postgres::types::ToSql + Sync),
                        sender as &(dyn tokio_postgres::types::ToSql + Sync),
                        receiver as &(dyn tokio_postgres::types::ToSql + Sync),
                        nonce as &(dyn tokio_postgres::types::ToSql + Sync),
                        fee_nanomina as &(dyn tokio_postgres::types::ToSql + Sync),
                        fee_payer as &(dyn tokio_postgres::types::ToSql + Sync),
                        amount_nanomina as &(dyn tokio_postgres::types::ToSql + Sync),
                        canonical as &(dyn tokio_postgres::types::ToSql + Sync),
                    ]
                },
            )
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
impl Actor for CanonicalUserCommandPersistenceActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.database_inserts
    }

    async fn handle_event(&self, event: Event) {
        if event.event_type == EventType::BatchCanonicalUserCommandLog {
            let log: BatchCanonicalUserCommandLogPayload = sonic_rs::from_str(&event.payload).unwrap();
            self.log_batch(&log).await.unwrap();
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
    use crate::event_sourcing::{
        events::{Event, EventType},
        mainnet_block_models::{CommandStatus, CommandSummary, CommandType},
    };
    use std::sync::Arc;
    use tokio::sync::broadcast;

    async fn setup_actor() -> (CanonicalUserCommandPersistenceActor, Arc<SharedPublisher>, broadcast::Receiver<Event>) {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let receiver = shared_publisher.subscribe();

        let actor = CanonicalUserCommandPersistenceActor::new(Arc::clone(&shared_publisher), &None).await;

        (actor, shared_publisher, receiver)
    }

    #[tokio::test]
    async fn test_insert_canonical_user_command_log_batch() {
        let (actor, _shared_publisher, _receiver) = setup_actor().await;

        // Prepare batch payload
        let payload = BatchCanonicalUserCommandLogPayload {
            height: 100,
            state_hash: "state_hash_100".to_string(),
            canonical: true,
            was_canonical: false,
            global_slot: 0,
            timestamp: 124312342142,
            commands: vec![CommandSummary {
                memo: "Test Memo 1".to_string(),
                fee_payer: "fee_payer_1".to_string(),
                sender: "sender_1".to_string(),
                receiver: "receiver_1".to_string(),
                status: CommandStatus::Applied,
                txn_type: CommandType::Payment,
                nonce: 42,
                fee_nanomina: 1000,
                amount_nanomina: 5000,
            }],
        };

        // Log the batch
        actor.log_batch(&payload).await.unwrap();

        // Query the database to verify the inserted command
        let query = "SELECT * FROM user_commands_log WHERE height = $1 AND state_hash = $2";
        let db_logger = actor.db_logger.lock().await;
        let row = db_logger
            .get_client()
            .query_one(query, &[&(payload.height as i64), &payload.state_hash])
            .await
            .unwrap();

        // Assert that the row matches the payload's command
        let command = &payload.commands[0];
        assert_eq!(row.get::<_, i64>("height"), payload.height as i64);
        assert_eq!(row.get::<_, String>("state_hash"), payload.state_hash);
        assert_eq!(row.get::<_, String>("sender"), command.sender);
        assert_eq!(row.get::<_, String>("receiver"), command.receiver);
        assert_eq!(row.get::<_, i64>("nonce"), command.nonce as i64);
        assert_eq!(row.get::<_, i64>("fee_nanomina"), command.fee_nanomina as i64);
        assert_eq!(row.get::<_, String>("fee_payer"), command.fee_payer);
        assert_eq!(row.get::<_, i64>("amount_nanomina"), command.amount_nanomina as i64);
        assert_eq!(row.get::<_, bool>("canonical"), payload.canonical);
    }

    #[tokio::test]
    async fn test_handle_event_canonical_user_command_log_batch() {
        let (actor, _, _) = setup_actor().await;

        // Prepare batch payload
        let batch_payload = BatchCanonicalUserCommandLogPayload {
            height: 200,
            state_hash: "state_hash_200".to_string(),
            canonical: false,
            was_canonical: true,
            global_slot: 0,
            timestamp: 124312342142,
            commands: vec![CommandSummary {
                memo: "Test Memo".to_string(),
                fee_payer: "fee_payer_2".to_string(),
                sender: "sender_2".to_string(),
                receiver: "receiver_2".to_string(),
                status: CommandStatus::Failed,
                txn_type: CommandType::Payment,
                nonce: 99,
                fee_nanomina: 2000,
                amount_nanomina: 10000,
            }],
        };

        // Serialize the batch payload into an event
        let event = Event {
            event_type: EventType::BatchCanonicalUserCommandLog,
            payload: sonic_rs::to_string(&batch_payload).unwrap(),
        };

        // Handle the event with the actor
        actor.handle_event(event).await;

        // Query the database to verify the inserted commands
        let query = "SELECT * FROM user_commands_log WHERE height = $1 AND state_hash = $2";
        let db_logger = actor.db_logger.lock().await;
        let row = db_logger
            .get_client()
            .query_one(query, &[&(batch_payload.height as i64), &batch_payload.state_hash])
            .await
            .unwrap();

        // Assert that the row matches the payload's command
        let command = &batch_payload.commands[0];
        assert_eq!(row.get::<_, i64>("height"), batch_payload.height as i64);
        assert_eq!(row.get::<_, String>("state_hash"), batch_payload.state_hash);
        assert_eq!(row.get::<_, String>("sender"), command.sender);
        assert_eq!(row.get::<_, String>("receiver"), command.receiver);
        assert_eq!(row.get::<_, i64>("nonce"), command.nonce as i64);
        assert_eq!(row.get::<_, i64>("fee_nanomina"), command.fee_nanomina as i64);
        assert_eq!(row.get::<_, String>("fee_payer"), command.fee_payer);
        assert_eq!(row.get::<_, i64>("amount_nanomina"), command.amount_nanomina as i64);
        assert_eq!(row.get::<_, bool>("canonical"), batch_payload.canonical);
    }

    #[tokio::test]
    async fn test_canonical_user_command_view_with_separate_batches() {
        // Set up the actor and database connection
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = CanonicalUserCommandPersistenceActor::new(Arc::clone(&shared_publisher), &None).await;
        let command = CommandSummary {
            memo: "Command 1".to_string(),
            fee_payer: "payer_1".to_string(),
            sender: "sender_1".to_string(),
            receiver: "receiver_1".to_string(),
            status: CommandStatus::Applied,
            txn_type: CommandType::Payment,
            nonce: 1,
            fee_nanomina: 500,
            amount_nanomina: 10000,
        };

        // First batch payload with the initial command
        let batch_payload_1 = BatchCanonicalUserCommandLogPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            canonical: false,
            was_canonical: false,
            global_slot: 0,
            timestamp: 124312342142,
            commands: vec![command.clone()],
        };

        // Second batch payload with the updated command
        let batch_payload_2 = BatchCanonicalUserCommandLogPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            canonical: true,
            was_canonical: true,
            global_slot: 0,
            timestamp: 124312342142,
            commands: vec![command],
        };

        // Insert the first batch into the database
        actor.log_batch(&batch_payload_1).await.unwrap();

        // Insert the second batch into the database
        actor.log_batch(&batch_payload_2).await.unwrap();

        // Query the database to ensure only the latest command is returned
        let query = "SELECT * FROM user_commands WHERE height = $1 AND state_hash = $2 ORDER BY timestamp DESC";
        let db_logger = actor.db_logger.lock().await;
        let rows = db_logger
            .get_client()
            .query(query, &[&(batch_payload_1.height as i64), &batch_payload_1.state_hash])
            .await
            .unwrap();

        // Ensure only the latest entry is returned
        assert_eq!(rows.len(), 1);

        let row = &rows[0];
        let latest_command = &batch_payload_2.commands[0]; // The latest command from the second batch

        // Verify the row matches the latest command
        assert_eq!(row.get::<_, i64>("height"), batch_payload_2.height as i64);
        assert_eq!(row.get::<_, String>("state_hash"), batch_payload_2.state_hash);
        assert_eq!(row.get::<_, String>("sender"), latest_command.sender);
        assert_eq!(row.get::<_, String>("receiver"), latest_command.receiver);
        assert_eq!(row.get::<_, i64>("nonce"), latest_command.nonce as i64);
        assert_eq!(row.get::<_, i64>("fee_nanomina"), latest_command.fee_nanomina as i64);
        assert_eq!(row.get::<_, String>("fee_payer"), latest_command.fee_payer);
        assert_eq!(row.get::<_, i64>("amount_nanomina"), latest_command.amount_nanomina as i64);
        assert_eq!(row.get::<_, bool>("canonical"), batch_payload_2.canonical);
    }

    #[tokio::test]
    async fn test_actor_height_event_published_with_batch() {
        // Set up the actor and shared publisher
        let (actor, _, mut receiver) = setup_actor().await;

        // Create a batch payload for canonical user commands
        let batch_payload = BatchCanonicalUserCommandLogPayload {
            height: 200,
            state_hash: "state_hash_200".to_string(),
            canonical: true,
            was_canonical: false,
            global_slot: 0,
            timestamp: 124312342142,
            commands: vec![CommandSummary {
                memo: "Test Memo".to_string(),
                fee_payer: "fee_payer_2".to_string(),
                sender: "sender_2".to_string(),
                receiver: "receiver_2".to_string(),
                status: CommandStatus::Applied,
                txn_type: CommandType::Payment,
                nonce: 43,
                fee_nanomina: 1500,
                amount_nanomina: 7500,
            }],
        };

        // Serialize the batch payload into an event
        let event = Event {
            event_type: EventType::BatchCanonicalUserCommandLog,
            payload: sonic_rs::to_string(&batch_payload).unwrap(),
        };

        // Handle the batch event
        actor.handle_event(event).await;

        // Listen for the `ActorHeight` event
        let received_event = receiver.recv().await.expect("Did not receive ActorHeight event");

        // Verify the event type is `ActorHeight`
        assert_eq!(received_event.event_type, EventType::ActorHeight);

        // Deserialize and validate the `ActorHeightPayload` content
        let actor_height_payload: ActorHeightPayload = sonic_rs::from_str(&received_event.payload).expect("Failed to deserialize ActorHeightPayload");
        assert_eq!(actor_height_payload.actor, actor.id());
        assert_eq!(actor_height_payload.height, batch_payload.height);
    }
}
