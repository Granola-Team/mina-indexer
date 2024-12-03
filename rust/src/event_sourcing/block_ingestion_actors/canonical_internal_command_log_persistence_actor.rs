use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    constants::POSTGRES_CONNECTION_STRING,
    event_sourcing::{
        db_logger::DbLogger,
        payloads::{ActorHeightPayload, CanonicalInternalCommandLogPayload},
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
}

impl CanonicalInternalCommandLogPersistenceActor {
    pub async fn new(shared_publisher: Arc<SharedPublisher>, root_node: &Option<(u64, String)>) -> Self {
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
        }
    }

    async fn log(&self, payload: &CanonicalInternalCommandLogPayload) -> Result<u64, &'static str> {
        let logger = self.db_logger.lock().await;
        match logger
            .insert(
                &[
                    &payload.internal_command_type.to_string(),
                    &(payload.height as i64),
                    &payload.state_hash,
                    &(payload.timestamp as i64),
                    &(payload.amount_nanomina as i64),
                    &payload.recipient,
                    &payload.canonical,
                ],
                payload.height,
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
            match self.log(&event_payload).await {
                Ok(affected_rows) => {
                    assert_eq!(affected_rows, 1);
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
        payloads::InternalCommandType,
    };
    use std::sync::Arc;
    use tokio::sync::broadcast;

    async fn setup_actor() -> (CanonicalInternalCommandLogPersistenceActor, Arc<SharedPublisher>, broadcast::Receiver<Event>) {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let receiver = shared_publisher.subscribe();

        let actor = CanonicalInternalCommandLogPersistenceActor::new(Arc::clone(&shared_publisher), &None).await;

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

        let affected_rows = actor.log(&payload).await.unwrap();

        // Validate that exactly one row was affected
        assert_eq!(affected_rows, 1);

        // Validate the data was correctly inserted into the table
        let query = "SELECT * FROM internal_commands_log WHERE height = $1 AND state_hash = $2 AND recipient = $3";
        let logger = actor.db_logger.lock().await;
        let row = logger
            .get_client()
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
        let query = "SELECT * FROM internal_commands_log WHERE height = $1 AND state_hash = $2 AND recipient = $3";
        let logger = actor.db_logger.lock().await;
        let row = logger
            .get_client()
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
        let actor = CanonicalInternalCommandLogPersistenceActor::new(Arc::clone(&shared_publisher), &None).await;

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

        actor.log(&payload1).await.unwrap();
        actor.log(&payload2).await.unwrap();

        // Query the internal_commands view
        let query = "SELECT * FROM internal_commands WHERE height = $1 AND state_hash = $2 AND recipient = $3";
        let logger = actor.db_logger.lock().await;
        let row = logger
            .get_client()
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

    #[tokio::test]
    async fn test_actor_height_event_published() {
        let (actor, _, mut receiver) = setup_actor().await;

        // Create a payload for a canonical internal command
        let payload = CanonicalInternalCommandLogPayload {
            internal_command_type: InternalCommandType::Coinbase,
            height: 150,
            state_hash: "state_hash_150".to_string(),
            timestamp: 1234567890,
            amount_nanomina: 10000,
            recipient: "recipient_150".to_string(),
            canonical: true,
            source: None,
            was_canonical: false,
        };

        // Create and publish the event
        let event = Event {
            event_type: EventType::CanonicalInternalCommandLog,
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

        println!(
            "ActorHeight event published: actor = {}, height = {}",
            actor_height_payload.actor, actor_height_payload.height
        );
    }
}
