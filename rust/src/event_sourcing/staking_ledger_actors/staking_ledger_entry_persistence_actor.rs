use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{constants::POSTGRES_CONNECTION_STRING, event_sourcing::payloads::StakingLedgerEntryPayload};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};
use tokio_postgres::{Client, NoTls};

pub struct StakingLedgerEntryPersistenceActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub client: Client,
    pub database_inserts: AtomicUsize,
}

impl StakingLedgerEntryPersistenceActor {
    pub async fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Unable to establish connection to database");

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        if let Err(e) = client.execute("DROP TABLE IF EXISTS staking_ledger CASCADE;", &[]).await {
            eprintln!("Unable to create staking_ledger table {e}");
        }

        let table_create = r#"
                CREATE TABLE IF NOT EXISTS staking_ledger (
                    entry_id BIGSERIAL PRIMARY KEY,
                    epoch BIGINT,
                    delegate TEXT,
                    stake BIGINT,
                    total_staked BIGINT,
                    delegators_count BIGINT
                );
            "#;
        if let Err(e) = client.execute(table_create, &[]).await {
            eprintln!("Unable to create staking_ledger table {e}");
        }

        Self {
            id: "StakingLedgerEntryPersistenceActor".to_string(),
            shared_publisher,
            client,
            database_inserts: AtomicUsize::new(0),
        }
    }

    async fn insert(&self, payload: &StakingLedgerEntryPayload) -> Result<(), &'static str> {
        self.client
            .execute(
                r#"
                INSERT INTO staking_ledger (epoch, delegate, stake, total_staked, delegators_count)
                VALUES ($1, $2, $3, $4, $5);
                "#,
                &[
                    &(payload.epoch as i64),
                    &payload.delegate,
                    &(payload.stake as i64),
                    &(payload.total_staked as i64),
                    &(payload.delegators_count as i64),
                ],
            )
            .await
            .map_err(|e| {
                eprintln!("Database insert error: {}", e);
                "Unable to insert into staking_ledger table"
            })?;

        Ok(())
    }
}

#[async_trait]
impl Actor for StakingLedgerEntryPersistenceActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.database_inserts
    }

    async fn handle_event(&self, event: Event) {
        if event.event_type == EventType::StakingLedgerEntry {
            let payload: StakingLedgerEntryPayload = sonic_rs::from_str(&event.payload).unwrap();
            self.insert(&payload).await.unwrap();
            self.database_inserts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[cfg(test)]
mod staking_ledger_entry_persistence_actor_tests {
    use super::*;
    use crate::event_sourcing::events::{Event, EventType};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_persistence_of_staking_ledger_entry() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = StakingLedgerEntryPersistenceActor::new(Arc::clone(&shared_publisher)).await;

        let payload = StakingLedgerEntryPayload {
            epoch: 10,
            delegate: "delegate_1".to_string(),
            stake: 1000000,
            total_staked: 5000000,
            delegators_count: 20,
        };

        actor
            .handle_event(Event {
                event_type: EventType::StakingLedgerEntry,
                payload: sonic_rs::to_string(&payload).unwrap(),
            })
            .await;

        // Assert that the database insert counter has incremented
        assert_eq!(actor.database_inserts.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
