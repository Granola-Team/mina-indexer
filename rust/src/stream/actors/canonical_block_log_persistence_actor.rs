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

pub struct CanonicalBlockLogPersistenceActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub client: Client,
    pub database_inserts: AtomicUsize,
}

impl CanonicalBlockLogPersistenceActor {
    pub async fn new(shared_publisher: Arc<SharedPublisher>, preserve_existing_data: bool) -> Self {
        if let Ok((client, connection)) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls).await {
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {}", e);
                }
            });

            if !preserve_existing_data {
                if let Err(e) = client.execute("DROP TABLE IF EXISTS canonical_block_log CASCADE;", &[]).await {
                    println!("Unable to drop canonical_block_log table {:?}", e);
                }
            }
            if let Err(e) = client
                .execute(
                    "CREATE TABLE IF NOT EXISTS canonical_block_log (
                        height BIGINT,
                        state_hash TEXT,
                        previous_state_hash TEXT,
                        user_command_count INTEGER,
                        snark_work_count INTEGER,
                        timestamp BIGINT,
                        coinbase_receiver TEXT,
                        coinbase_reward_nanomina BIGINT,
                        global_slot_since_genesis BIGINT,
                        last_vrf_output TEXT,
                        is_berkeley_block BOOLEAN,
                        canonical BOOLEAN,
                        entry_id BIGSERIAL PRIMARY KEY,
                        UNIQUE (height, state_hash, timestamp)
                    );",
                    &[],
                )
                .await
            {
                println!("Unable to create canonical_block_log table {:?}", e);
            }
            if let Err(e) = client
                .execute(
                    "CREATE OR REPLACE VIEW canonical_blocks AS
                    SELECT DISTINCT ON (height, state_hash) *
                    FROM canonical_block_log
                    ORDER BY height, state_hash, entry_id DESC;",
                    &[],
                )
                .await
            {
                println!("Unable to create canonical_block_log table {:?}", e);
            }
            Self {
                id: "CanonicalBlockLogActor".to_string(),
                shared_publisher,
                client,
                database_inserts: AtomicUsize::new(0),
            }
        } else {
            panic!("Unable to establish connection to database");
        }
    }

    async fn insert_canonical_block_log(&self, payload: &CanonicalBlockLogPayload) -> Result<(), &'static str> {
        let insert_query = r#"
            INSERT INTO canonical_block_log (
                height, state_hash, previous_state_hash, user_command_count, snark_work_count,
                timestamp, coinbase_receiver, coinbase_reward_nanomina, global_slot_since_genesis,
                last_vrf_output, is_berkeley_block, canonical
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12
            ) ON CONFLICT DO NOTHING
        "#;

        self.client
            .execute(
                insert_query,
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
            self.insert_canonical_block_log(&log).await.unwrap();
        }
    }

    fn publish(&self, event: Event) {
        self.shared_publisher.publish(event);
    }
}

#[cfg(test)]
mod canonical_block_log_persistence_tests {
    use super::*;
    use crate::stream::events::{Event, EventType};
    use std::sync::Arc;
    use tokio::sync::broadcast;

    async fn setup_actor() -> (CanonicalBlockLogPersistenceActor, Arc<SharedPublisher>, broadcast::Receiver<Event>) {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let receiver = shared_publisher.subscribe();

        let actor = CanonicalBlockLogPersistenceActor::new(Arc::clone(&shared_publisher), false).await;

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

        actor.insert_canonical_block_log(&payload).await.unwrap();

        let query = "SELECT * FROM canonical_block_log WHERE height = $1 AND state_hash = $2 AND timestamp = $3";
        let row = actor
            .client
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

        let query = "SELECT * FROM canonical_block_log WHERE height = $1 AND state_hash = $2 AND timestamp = $3";
        let row = actor
            .client
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
    async fn test_canonical_blocks_view() {
        // Set up the actor and database connection
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = CanonicalBlockLogPersistenceActor::new(Arc::clone(&shared_publisher), false).await;

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

        let payload2 = CanonicalBlockLogPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            previous_state_hash: "prev_hash".to_string(),
            user_command_count: 12,
            snark_work_count: 3,
            timestamp: 1234567891, // Later timestamp
            coinbase_receiver: "receiver_1".to_string(),
            coinbase_reward_nanomina: 1100,
            global_slot_since_genesis: 101,
            last_vrf_output: "vrf_output_2".to_string(),
            is_berkeley_block: true,
            canonical: true,
        };

        // Insert the payloads into the database
        actor.insert_canonical_block_log(&payload1).await.unwrap();
        actor.insert_canonical_block_log(&payload2).await.unwrap();

        // Query the canonical_blocks view
        let query = "SELECT * FROM canonical_blocks WHERE height = $1 AND state_hash = $2";
        let rows = actor.client.query(query, &[&(payload1.height as i64), &payload1.state_hash]).await.unwrap();

        // Ensure only one row is returned
        assert_eq!(rows.len(), 1);

        // Validate the returned row matches the payload with the highest entry_id
        let row = &rows[0];
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
}
