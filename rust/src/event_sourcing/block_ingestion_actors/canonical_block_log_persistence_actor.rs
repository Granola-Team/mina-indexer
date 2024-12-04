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

pub struct CanonicalBlockLogPersistenceActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub db_logger: Arc<Mutex<DbLogger>>,
    pub database_inserts: AtomicUsize,
}

impl CanonicalBlockLogPersistenceActor {
    pub async fn new(shared_publisher: Arc<SharedPublisher>, root_node: &Option<(u64, String)>) -> Self {
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Unable to establish connection to database");
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("connection error: {}", e);
            }
        });

        let logger = DbLogger::builder(client)
            .name("blocks")
            .add_column("height BIGINT")
            .add_column("state_hash TEXT")
            .add_column("previous_state_hash TEXT")
            .add_column("user_command_count INTEGER")
            .add_column("snark_work_count INTEGER")
            .add_column("timestamp BIGINT")
            .add_column("coinbase_receiver TEXT")
            .add_column("coinbase_reward_nanomina BIGINT")
            .add_column("global_slot_since_genesis BIGINT")
            .add_column("last_vrf_output TEXT")
            .add_column("is_berkeley_block BOOLEAN")
            .add_column("canonical BOOLEAN")
            .distinct_columns(&["height", "state_hash"])
            .build(root_node)
            .await
            .expect("Failed to build blocks_log and blocks view");

        if let Some((height, state_hash)) = root_node {
            if let Err(e) = logger
                .get_client()
                .execute(
                    "DELETE FROM blocks_log WHERE height > $1 AND (height = $1 AND state_hash = $2)",
                    &[&(height.to_owned() as i64), state_hash],
                )
                .await
            {
                error!("Unable to drop data: {}", e);
            }
        }

        Self {
            id: "CanonicalBlockLogActor".to_string(),
            shared_publisher,
            db_logger: Arc::new(Mutex::new(logger)),
            database_inserts: AtomicUsize::new(0),
        }
    }

    async fn log(&self, payload: &CanonicalBlockLogPayload) -> Result<(), &'static str> {
        let logger = self.db_logger.lock().await;
        logger
            .insert(
                &[
                    &(payload.height as i64),
                    &payload.state_hash,
                    &payload.previous_state_hash,
                    &(payload.user_command_count as i32),
                    &(payload.snark_work_count as i32),
                    &(payload.timestamp as i64),
                    &payload.coinbase_receiver,
                    &(payload.coinbase_reward_nanomina as i64),
                    &(payload.global_slot_since_genesis as i64),
                    &payload.last_vrf_output,
                    &payload.is_berkeley_block,
                    &payload.canonical,
                ],
                payload.height,
            )
            .await
            .map_err(|_| "Unable to insert into canonical_block_log table")?;

        Ok(())
    }
}

#[async_trait]
impl Actor for CanonicalBlockLogPersistenceActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.database_inserts
    }

    async fn handle_event(&self, event: Event) {
        if event.event_type == EventType::CanonicalBlockLog {
            let log: CanonicalBlockLogPayload = sonic_rs::from_str(&event.payload).unwrap();
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
mod canonical_block_log_persistence_tests {
    use super::*;
    use crate::event_sourcing::events::{Event, EventType};
    use std::sync::Arc;
    use tokio::sync::broadcast;

    async fn setup_actor() -> (CanonicalBlockLogPersistenceActor, Arc<SharedPublisher>, broadcast::Receiver<Event>) {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let receiver = shared_publisher.subscribe();

        let actor = CanonicalBlockLogPersistenceActor::new(Arc::clone(&shared_publisher), &None).await;

        (actor, shared_publisher, receiver)
    }

    #[tokio::test]
    async fn test_insert_canonical_block_log() {
        let (actor, _shared_publisher, _receiver) = setup_actor().await;

        let payload = CanonicalBlockLogPayload {
            height: 100,
            state_hash: "state_hash_100".to_string(),
            previous_state_hash: "state_hash_99".to_string(),
            user_command_count: 5,
            snark_work_count: 3,
            timestamp: 1234567890,
            coinbase_receiver: "coinbase_receiver".to_string(),
            coinbase_reward_nanomina: 1000,
            global_slot_since_genesis: 50,
            last_vrf_output: "vrf_output".to_string(),
            is_berkeley_block: true,
            canonical: true,
        };

        actor.log(&payload).await.unwrap();

        let query = "SELECT * FROM blocks_log WHERE height = $1 AND state_hash = $2 AND timestamp = $3";
        let db_logger = actor.db_logger.lock().await;
        let row = db_logger
            .get_client()
            .query_one(query, &[&(payload.height as i64), &payload.state_hash, &(payload.timestamp as i64)])
            .await
            .unwrap();

        assert_eq!(row.get::<_, i64>("height"), payload.height as i64);
        assert_eq!(row.get::<_, String>("state_hash"), payload.state_hash);
        assert_eq!(row.get::<_, String>("previous_state_hash"), payload.previous_state_hash);
        assert_eq!(row.get::<_, i32>("user_command_count"), payload.user_command_count as i32);
        assert_eq!(row.get::<_, i32>("snark_work_count"), payload.snark_work_count as i32);
        assert_eq!(row.get::<_, i64>("timestamp"), payload.timestamp as i64);
        assert_eq!(row.get::<_, String>("coinbase_receiver"), payload.coinbase_receiver);
        assert_eq!(row.get::<_, i64>("coinbase_reward_nanomina"), payload.coinbase_reward_nanomina as i64);
        assert_eq!(row.get::<_, i64>("global_slot_since_genesis"), payload.global_slot_since_genesis as i64);
        assert_eq!(row.get::<_, String>("last_vrf_output"), payload.last_vrf_output);
        assert_eq!(row.get::<_, bool>("is_berkeley_block"), payload.is_berkeley_block);
        assert_eq!(row.get::<_, bool>("canonical"), payload.canonical);
    }

    #[tokio::test]
    async fn test_handle_event_canonical_block_log() {
        let (actor, _, _) = setup_actor().await;

        let payload = CanonicalBlockLogPayload {
            height: 200,
            state_hash: "state_hash_200".to_string(),
            previous_state_hash: "state_hash_199".to_string(),
            user_command_count: 7,
            snark_work_count: 2,
            timestamp: 987654321,
            coinbase_receiver: "another_receiver".to_string(),
            coinbase_reward_nanomina: 2000,
            global_slot_since_genesis: 100,
            last_vrf_output: "another_vrf_output".to_string(),
            is_berkeley_block: false,
            canonical: false,
        };

        let event = Event {
            event_type: EventType::CanonicalBlockLog,
            payload: sonic_rs::to_string(&payload).unwrap(),
        };

        actor.handle_event(event).await;

        let query = "SELECT * FROM blocks_log WHERE height = $1 AND state_hash = $2 AND timestamp = $3";
        let db_logger = actor.db_logger.lock().await;
        let row = db_logger
            .get_client()
            .query_one(query, &[&(payload.height as i64), &payload.state_hash, &(payload.timestamp as i64)])
            .await
            .unwrap();

        assert_eq!(row.get::<_, i64>("height"), payload.height as i64);
        assert_eq!(row.get::<_, String>("state_hash"), payload.state_hash);
        assert_eq!(row.get::<_, String>("previous_state_hash"), payload.previous_state_hash);
        assert_eq!(row.get::<_, i32>("user_command_count"), payload.user_command_count as i32);
        assert_eq!(row.get::<_, i32>("snark_work_count"), payload.snark_work_count as i32);
        assert_eq!(row.get::<_, i64>("timestamp"), payload.timestamp as i64);
        assert_eq!(row.get::<_, String>("coinbase_receiver"), payload.coinbase_receiver);
        assert_eq!(row.get::<_, i64>("coinbase_reward_nanomina"), payload.coinbase_reward_nanomina as i64);
        assert_eq!(row.get::<_, i64>("global_slot_since_genesis"), payload.global_slot_since_genesis as i64);
        assert_eq!(row.get::<_, String>("last_vrf_output"), payload.last_vrf_output);
        assert_eq!(row.get::<_, bool>("is_berkeley_block"), payload.is_berkeley_block);
        assert_eq!(row.get::<_, bool>("canonical"), payload.canonical);
    }

    #[tokio::test]
    async fn test_canonical_blocks_canonical_block_log() {
        // Set up the actor and database connection
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = CanonicalBlockLogPersistenceActor::new(Arc::clone(&shared_publisher), &None).await;

        // Insert multiple entries for the same (height, state_hash) with different canonicities
        let payload1 = CanonicalBlockLogPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            previous_state_hash: "prev_hash".to_string(),
            user_command_count: 10,
            snark_work_count: 2,
            timestamp: 1234567890,
            coinbase_receiver: "receiver_1".to_string(),
            coinbase_reward_nanomina: 1000,
            global_slot_since_genesis: 100,
            last_vrf_output: "vrf_output_1".to_string(),
            is_berkeley_block: false,
            canonical: false,
        };

        let mut payload2 = payload1.clone();
        payload2.canonical = true;

        // Insert the payloads into the database
        actor.log(&payload1).await.unwrap();
        actor.log(&payload1).await.unwrap();
        actor.log(&payload2).await.unwrap();

        // Query the canonical_block_log view
        let query = "SELECT * FROM blocks WHERE height = $1";
        let db_logger = actor.db_logger.lock().await;
        let rows = db_logger.get_client().query(query, &[&1_i64]).await.unwrap();
        assert_eq!(rows.len(), 1);

        let row_last = rows.last().unwrap();

        // Validate the returned row matches the payload with the highest entry_id
        assert_eq!(row_last.get::<_, i64>("height"), payload2.height as i64);
        assert_eq!(row_last.get::<_, String>("state_hash"), payload2.state_hash);
        assert_eq!(row_last.get::<_, String>("previous_state_hash"), payload2.previous_state_hash);
        assert_eq!(row_last.get::<_, i32>("user_command_count"), payload2.user_command_count as i32);
        assert_eq!(row_last.get::<_, i32>("snark_work_count"), payload2.snark_work_count as i32);
        assert_eq!(row_last.get::<_, i64>("timestamp"), payload2.timestamp as i64);
        assert_eq!(row_last.get::<_, String>("coinbase_receiver"), payload2.coinbase_receiver);
        assert_eq!(row_last.get::<_, i64>("coinbase_reward_nanomina"), payload2.coinbase_reward_nanomina as i64);
        assert_eq!(row_last.get::<_, i64>("global_slot_since_genesis"), payload2.global_slot_since_genesis as i64);
        assert_eq!(row_last.get::<_, String>("last_vrf_output"), payload2.last_vrf_output);
        assert_eq!(row_last.get::<_, bool>("is_berkeley_block"), payload2.is_berkeley_block);
        assert_eq!(row_last.get::<_, bool>("canonical"), payload2.canonical);
    }

    #[tokio::test]
    async fn test_canonical_blocks_view() {
        // Set up the actor and database connection
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = CanonicalBlockLogPersistenceActor::new(Arc::clone(&shared_publisher), &None).await;

        // Insert multiple entries for the same (height, state_hash) with different canonicities
        let payload1 = CanonicalBlockLogPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            previous_state_hash: "prev_hash".to_string(),
            user_command_count: 10,
            snark_work_count: 2,
            timestamp: 1234567890,
            coinbase_receiver: "receiver_1".to_string(),
            coinbase_reward_nanomina: 1000,
            global_slot_since_genesis: 100,
            last_vrf_output: "vrf_output_1".to_string(),
            is_berkeley_block: false,
            canonical: false,
        };

        let mut payload2 = payload1.clone();
        payload2.canonical = true;

        // Insert the payloads into the database
        actor.log(&payload1).await.unwrap();
        actor.log(&payload2).await.unwrap();

        // Query the blocks view
        let query = "SELECT * FROM blocks WHERE height = $1";
        let db_logger = actor.db_logger.lock().await;
        let row = db_logger.get_client().query_one(query, &[&1_i64]).await.unwrap();

        // Validate the returned row matches the payload with the highest entry_id
        assert_eq!(row.get::<_, i64>("height"), payload2.height as i64);
        assert_eq!(row.get::<_, String>("state_hash"), payload2.state_hash);
        assert_eq!(row.get::<_, String>("previous_state_hash"), payload2.previous_state_hash);
        assert_eq!(row.get::<_, i32>("user_command_count"), payload2.user_command_count as i32);
        assert_eq!(row.get::<_, i32>("snark_work_count"), payload2.snark_work_count as i32);
        assert_eq!(row.get::<_, i64>("timestamp"), payload2.timestamp as i64);
        assert_eq!(row.get::<_, String>("coinbase_receiver"), payload2.coinbase_receiver);
        assert_eq!(row.get::<_, i64>("coinbase_reward_nanomina"), payload2.coinbase_reward_nanomina as i64);
        assert_eq!(row.get::<_, i64>("global_slot_since_genesis"), payload2.global_slot_since_genesis as i64);
        assert_eq!(row.get::<_, String>("last_vrf_output"), payload2.last_vrf_output);
        assert_eq!(row.get::<_, bool>("is_berkeley_block"), payload2.is_berkeley_block);
        assert_eq!(row.get::<_, bool>("canonical"), payload2.canonical);
    }

    #[tokio::test]
    async fn test_actor_height_event_published() {
        let (actor, _, mut receiver) = setup_actor().await;

        // Create a payload for a canonical block log
        let payload = CanonicalBlockLogPayload {
            height: 300,
            state_hash: "state_hash_300".to_string(),
            previous_state_hash: "state_hash_299".to_string(),
            user_command_count: 15,
            snark_work_count: 5,
            timestamp: 1627891234,
            coinbase_receiver: "receiver_300".to_string(),
            coinbase_reward_nanomina: 3000,
            global_slot_since_genesis: 120,
            last_vrf_output: "vrf_output_300".to_string(),
            is_berkeley_block: false,
            canonical: true,
        };

        // Create and publish the event
        let event = Event {
            event_type: EventType::CanonicalBlockLog,
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
