use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    constants::POSTGRES_CONNECTION_STRING,
    event_sourcing::{
        db_logger::DbLogger,
        payloads::{ActorHeightPayload, BulkCanonicalInternalCommandLogPayload},
    },
};
use anyhow::Result;
use async_trait::async_trait;
use futures::lock::Mutex;
use std::sync::{atomic::AtomicUsize, Arc};
use tokio_postgres::NoTls;

pub struct CanonicalInternalCommandLogPersistenceActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub database_inserts: AtomicUsize,
    pub db_logger: Arc<Mutex<DbLogger>>,
    pub modulo_10: u64,
}

impl CanonicalInternalCommandLogPersistenceActor {
    pub async fn new(shared_publisher: Arc<SharedPublisher>, root_node: &Option<(u64, String)>, modulo_10: u64) -> Self {
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Unable to connect to database");
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        let logger = DbLogger::builder(client)
            .name("internal_commands")
            .add_column("internal_command_type TEXT NOT NULL")
            .add_column("height BIGINT NOT NULL")
            .add_column("state_hash TEXT NOT NULL")
            .add_column("timestamp BIGINT NOT NULL")
            .add_column("amount_nanomina BIGINT NOT NULL")
            .add_column("recipient TEXT NOT NULL")
            .add_column("is_canonical BOOLEAN NOT NULL")
            .distinct_columns(&["height", "internal_command_type", "state_hash", "recipient", "amount_nanomina"])
            .build(root_node)
            .await
            .expect("Failed to build internal_commands_log and internal_commands view");

        Self {
            id: "CanonicalInternalCommandLogPersistenceActor".to_string(),
            shared_publisher,
            db_logger: Arc::new(Mutex::new(logger)),
            database_inserts: AtomicUsize::new(0),
            modulo_10,
        }
    }

    async fn log_batch(&self, payload: &BulkCanonicalInternalCommandLogPayload) -> Result<(), &'static str> {
        let mut values = Vec::new();

        // Pre-allocate values for all rows
        for command in &payload.commands {
            let internal_command_type_str = command.internal_command_type.to_string();

            values.push((
                internal_command_type_str,
                payload.height as i64,
                payload.state_hash.clone(),
                command.timestamp as i64,
                command.amount_nanomina as i64,
                command.recipient.clone(),
                payload.canonical,
            ));
        }

        // Create rows referencing pre-allocated values
        let rows: Vec<Vec<&(dyn tokio_postgres::types::ToSql + Sync)>> = values
            .iter()
            .map(
                |(internal_command_type, height, state_hash, timestamp, amount_nanomina, recipient, canonical)| {
                    vec![
                        internal_command_type as &(dyn tokio_postgres::types::ToSql + Sync),
                        height as &(dyn tokio_postgres::types::ToSql + Sync),
                        state_hash as &(dyn tokio_postgres::types::ToSql + Sync),
                        timestamp as &(dyn tokio_postgres::types::ToSql + Sync),
                        amount_nanomina as &(dyn tokio_postgres::types::ToSql + Sync),
                        recipient as &(dyn tokio_postgres::types::ToSql + Sync),
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
impl Actor for CanonicalInternalCommandLogPersistenceActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.database_inserts
    }

    async fn handle_event(&self, event: Event) {
        if event.event_type == EventType::BulkCanonicalInternalCommandLog {
            let event_payload: BulkCanonicalInternalCommandLogPayload = sonic_rs::from_str(&event.payload).unwrap();
            if event_payload.height % 10 != self.modulo_10 {
                return;
            }
            match self.log_batch(&event_payload).await {
                Ok(_) => {
                    self.shared_publisher.incr_database_insert();
                    self.actor_outputs().fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
mod canonical_internal_command_log_tests {
    use super::*;
    use crate::event_sourcing::{
        events::{Event, EventType},
        payloads::{InternalCommandSubPayload, InternalCommandType},
    };
    use std::sync::Arc;
    use tokio::sync::broadcast;

    async fn setup_actor() -> (
        Vec<CanonicalInternalCommandLogPersistenceActor>,
        Arc<SharedPublisher>,
        broadcast::Receiver<Event>,
    ) {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let receiver = shared_publisher.subscribe();

        let mut actors = vec![];
        for i in 0..=9 {
            actors.push(CanonicalInternalCommandLogPersistenceActor::new(Arc::clone(&shared_publisher), &None, i).await);
        }

        (actors, shared_publisher, receiver)
    }

    #[tokio::test]
    async fn test_insert_canonical_internal_command_log() {
        let (actors, _shared_publisher, _receiver) = setup_actor().await;

        // Prepare the payload
        let payload = BulkCanonicalInternalCommandLogPayload {
            height: 100,
            state_hash: "state_hash_100".to_string(),
            canonical: true,
            was_canonical: false,
            commands: vec![InternalCommandSubPayload {
                internal_command_type: InternalCommandType::FeeTransfer,
                timestamp: 1234567890,
                amount_nanomina: 500,
                recipient: "recipient_1".to_string(),
            }],
        };

        // Log the batch
        let result = actors[0].log_batch(&payload).await;
        assert!(result.is_ok(), "log_batch should succeed");

        // Validate the data was correctly inserted into the table
        let query = "SELECT * FROM internal_commands_log WHERE height = $1 AND state_hash = $2 AND recipient = $3";
        let logger = actors[0].db_logger.lock().await;
        let row = logger
            .get_client()
            .query_one(query, &[&(payload.height as i64), &payload.state_hash, &payload.commands[0].recipient])
            .await
            .unwrap();

        let command = &payload.commands[0];
        assert_eq!(row.get::<_, String>("internal_command_type"), command.internal_command_type.to_string());
        assert_eq!(row.get::<_, i64>("height"), payload.height as i64);
        assert_eq!(row.get::<_, String>("state_hash"), payload.state_hash);
        assert_eq!(row.get::<_, i64>("timestamp"), command.timestamp as i64);
        assert_eq!(row.get::<_, i64>("amount_nanomina"), command.amount_nanomina as i64);
        assert_eq!(row.get::<_, String>("recipient"), command.recipient);
        assert_eq!(row.get::<_, bool>("is_canonical"), payload.canonical);
    }

    #[tokio::test]
    async fn test_handle_event_canonical_internal_command_log() {
        let (actors, _, _) = setup_actor().await;

        // Prepare the payload
        let bulk_payload = BulkCanonicalInternalCommandLogPayload {
            height: 200,
            state_hash: "state_hash_200".to_string(),
            canonical: false,
            was_canonical: false,
            commands: vec![InternalCommandSubPayload {
                internal_command_type: InternalCommandType::Coinbase,
                timestamp: 987654321,
                amount_nanomina: 1000,
                recipient: "recipient_2".to_string(),
            }],
        };

        // Convert the payload into an event
        let event = Event {
            event_type: EventType::BulkCanonicalInternalCommandLog,
            payload: sonic_rs::to_string(&bulk_payload).unwrap(),
        };

        // Handle the event
        actors[0].handle_event(event).await;

        // Validate the data was correctly inserted into the table
        let query = "SELECT * FROM internal_commands_log WHERE height = $1 AND state_hash = $2 AND recipient = $3";
        let logger = actors[0].db_logger.lock().await;
        let row = logger
            .get_client()
            .query_one(
                query,
                &[&(bulk_payload.height as i64), &bulk_payload.state_hash, &bulk_payload.commands[0].recipient],
            )
            .await
            .unwrap();

        // Extract the command for validation
        let command = &bulk_payload.commands[0];
        assert_eq!(row.get::<_, String>("internal_command_type"), command.internal_command_type.to_string());
        assert_eq!(row.get::<_, i64>("height"), bulk_payload.height as i64);
        assert_eq!(row.get::<_, String>("state_hash"), bulk_payload.state_hash);
        assert_eq!(row.get::<_, i64>("timestamp"), command.timestamp as i64);
        assert_eq!(row.get::<_, i64>("amount_nanomina"), command.amount_nanomina as i64);
        assert_eq!(row.get::<_, String>("recipient"), command.recipient);
        assert_eq!(row.get::<_, bool>("is_canonical"), bulk_payload.canonical);
    }

    #[tokio::test]
    async fn test_canonical_internal_commands_view() {
        // Set up the actor and database connection
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = CanonicalInternalCommandLogPersistenceActor::new(Arc::clone(&shared_publisher), &None, 1).await;

        // First batch payload
        let bulk_payload1 = BulkCanonicalInternalCommandLogPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            canonical: false,
            was_canonical: false,
            commands: vec![InternalCommandSubPayload {
                internal_command_type: InternalCommandType::FeeTransfer,
                timestamp: 1234567890,
                amount_nanomina: 1000,
                recipient: "recipient_1".to_string(),
            }],
        };

        // Second batch payload
        let bulk_payload2 = BulkCanonicalInternalCommandLogPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            canonical: true,
            was_canonical: false,
            commands: vec![InternalCommandSubPayload {
                internal_command_type: InternalCommandType::FeeTransfer,
                timestamp: 1234567891, // Later timestamp
                amount_nanomina: 1000,
                recipient: "recipient_1".to_string(),
            }],
        };

        // Insert each batch separately
        actor.log_batch(&bulk_payload1).await.unwrap();
        actor.log_batch(&bulk_payload2).await.unwrap();

        // Query the internal_commands view
        let query = "SELECT * FROM internal_commands WHERE height = $1 AND state_hash = $2 AND recipient = $3";
        let logger = actor.db_logger.lock().await;
        let row = logger
            .get_client()
            .query_one(
                query,
                &[&(bulk_payload1.height as i64), &bulk_payload1.state_hash, &bulk_payload1.commands[0].recipient],
            )
            .await
            .unwrap();

        // Validate that the row returned matches the payload with the highest timestamp
        let command2 = &bulk_payload2.commands[0];
        assert_eq!(row.get::<_, String>("internal_command_type"), command2.internal_command_type.to_string());
        assert_eq!(row.get::<_, i64>("height"), bulk_payload2.height as i64);
        assert_eq!(row.get::<_, String>("state_hash"), bulk_payload2.state_hash);
        assert_eq!(row.get::<_, i64>("timestamp"), command2.timestamp as i64);
        assert_eq!(row.get::<_, i64>("amount_nanomina"), command2.amount_nanomina as i64);
        assert_eq!(row.get::<_, String>("recipient"), command2.recipient);
        assert_eq!(row.get::<_, bool>("is_canonical"), bulk_payload2.canonical);
    }

    #[tokio::test]
    async fn test_actor_height_event_published() {
        let (actors, _, mut receiver) = setup_actor().await;

        // Create a bulk payload for a canonical internal command
        let bulk_payload = BulkCanonicalInternalCommandLogPayload {
            height: 150,
            state_hash: "state_hash_150".to_string(),
            canonical: true,
            was_canonical: false,
            commands: vec![InternalCommandSubPayload {
                internal_command_type: InternalCommandType::Coinbase,
                timestamp: 1234567890,
                amount_nanomina: 10000,
                recipient: "recipient_150".to_string(),
            }],
        };

        let event = Event {
            event_type: EventType::BulkCanonicalInternalCommandLog,
            payload: sonic_rs::to_string(&bulk_payload).unwrap(),
        };
        actors[0].handle_event(event).await;

        // Listen for the `ActorHeight` event
        let received_event = tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv())
            .await
            .expect("Expected a published event")
            .expect("Event received");

        // Verify the event type is `ActorHeight`
        assert_eq!(received_event.event_type, EventType::ActorHeight);

        // Deserialize the payload
        let actor_height_payload: ActorHeightPayload = sonic_rs::from_str(&received_event.payload).unwrap();

        // Validate the `ActorHeightPayload` content
        assert_eq!(actor_height_payload.actor, actors[0].id());
        assert_eq!(actor_height_payload.height, bulk_payload.height);

        println!(
            "ActorHeight event published: actor = {}, height = {}",
            actor_height_payload.actor, actor_height_payload.height
        );
    }
}
