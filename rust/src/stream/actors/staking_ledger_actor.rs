use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    constants::POSTGRES_CONNECTION_STRING,
    stream::payloads::{AccountingEntry, AccountingEntryType, ActorHeightPayload, DoubleEntryRecordPayload, LedgerDestination},
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};
use tokio_postgres::{Client, NoTls};

pub struct StakingLedgerActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub database_inserts: AtomicUsize,
    pub client: Client,
}

impl StakingLedgerActor {
    pub async fn new(shared_publisher: Arc<SharedPublisher>, root_node: &Option<(u64, String)>) -> Self {
        if let Ok((client, connection)) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls).await {
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {}", e);
                }
            });

            if let Some((height, _)) = root_node {
                if let Err(e) = client
                    .execute("DELETE FROM staking_ledger WHERE height >= $1;", &[&(height.to_owned() as i64)])
                    .await
                {
                    println!("Unable to drop staking_ledger table {:?}", e);
                }
            } else if let Err(e) = client.execute("DROP TABLE IF EXISTS staking_ledger CASCADE;", &[]).await {
                println!("Unable to drop staking_ledger table {:?}", e);
            }

            if let Err(e) = client
                .execute(
                    "CREATE TABLE IF NOT EXISTS staking_ledger (
                        address TEXT NOT NULL,
                        counterparty TEXT NOT NULL,
                        stake_delta BIGINT NOT NULL,
                        epoch BIGINT NOT NULL,
                        height BIGINT NOT NULL,
                        state_hash TEXT NOT NULL,
                        entry_id BIGSERIAL PRIMARY KEY
                    );",
                    &[],
                )
                .await
            {
                println!("Unable to create staking_ledger table {:?}", e);
            }

            if let Err(e) = client
                .execute(
                    "CREATE OR REPLACE VIEW staking_summary AS
                    SELECT
                        address,
                        epoch,
                        SUM(stake_delta) AS total_stake
                    FROM
                        staking_ledger
                    GROUP BY
                        address, epoch
                    ORDER BY
                        epoch DESC;",
                    &[],
                )
                .await
            {
                println!("Unable to create staking_summary view {:?}", e);
            }

            Self {
                id: "StakingLedgerActor".to_string(),
                shared_publisher,
                client,
                database_inserts: AtomicUsize::new(0),
            }
        } else {
            panic!("Unable to establish connection to database")
        }
    }

    async fn log_staking_entry(&self, payload: &AccountingEntry, height: &i64, state_hash: &str, epoch: &i64) -> Result<u64, &'static str> {
        let insert_query = r#"
            INSERT INTO staking_ledger (address, counterparty, stake_delta, epoch, height, state_hash)
            VALUES ($1, $2, $3, $4, $5, $6);
        "#;

        let stake_delta: i64 = match payload.entry_type {
            AccountingEntryType::Credit => payload.amount_nanomina as i64,
            AccountingEntryType::Debit => -(payload.amount_nanomina as i64),
        };

        match self
            .client
            .execute(
                insert_query,
                &[&payload.account, &payload.counterparty, &stake_delta, epoch, height, &state_hash.to_owned()],
            )
            .await
        {
            Err(e) => {
                let msg = e.to_string();
                println!("{}", msg);
                Err("unable to insert into staking_ledger table")
            }
            Ok(affected_rows) => Ok(affected_rows),
        }
    }
}

#[async_trait]
impl Actor for StakingLedgerActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.database_inserts
    }

    async fn handle_event(&self, event: Event) {
        if event.event_type == EventType::DoubleEntryTransaction {
            let event_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
            if event_payload.ledger_destination != LedgerDestination::StakingLedger {
                return;
            }

            for accounting_entry in event_payload.lhs.iter().chain(event_payload.rhs.iter()) {
                match self
                    .log_staking_entry(
                        accounting_entry,
                        &(event_payload.height as i64),
                        &event_payload.state_hash,
                        &(accounting_entry.timestamp as i64),
                    )
                    .await
                {
                    Ok(affected_rows) => {
                        assert_eq!(affected_rows, 1);
                        self.shared_publisher.incr_database_insert();
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
                        panic!("{:?}", e);
                    }
                }
            }
        }
    }

    fn publish(&self, event: Event) {
        self.shared_publisher.publish(event);
    }
}

#[cfg(test)]
mod staking_ledger_actor_tests {
    use super::*;
    use crate::stream::payloads::{AccountingEntry, AccountingEntryAccountType, AccountingEntryType, DoubleEntryRecordPayload, LedgerDestination};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_staking_entry_insertion() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = StakingLedgerActor::new(shared_publisher, &None).await;

        let accounting_entry = AccountingEntry {
            entry_type: AccountingEntryType::Credit,
            account: "staking_account".to_string(),
            account_type: AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: 1000,
            timestamp: 123456789,
            counterparty: "counterparty_1".to_string(),
            transfer_type: "StakeDelegation".to_string(),
        };

        let height: i64 = 10;
        let state_hash = "test_state_hash";
        let epoch: i64 = 1;

        let result = actor.log_staking_entry(&accounting_entry, &height, state_hash, &epoch).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        let rows = actor
            .client
            .query("SELECT * FROM staking_ledger WHERE address = $1", &[&accounting_entry.account])
            .await
            .expect("Failed to query database");

        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.get::<_, String>("address"), accounting_entry.account);
        assert_eq!(row.get::<_, String>("counterparty"), "counterparty_1");
        assert_eq!(row.get::<_, i64>("stake_delta"), 1000);
        assert_eq!(row.get::<_, i64>("epoch"), epoch);
        assert_eq!(row.get::<_, i64>("height"), height);
        assert_eq!(row.get::<_, String>("state_hash"), state_hash);
    }

    #[tokio::test]
    async fn test_only_staking_ledger_entries_processed() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = StakingLedgerActor::new(shared_publisher, &None).await;

        // Create a non-staking ledger event payload
        let non_staking_payload = DoubleEntryRecordPayload {
            height: 10,
            state_hash: "non_staking_state_hash".to_string(),
            ledger_destination: LedgerDestination::BlockchainLedger, // Non-staking destination
            lhs: vec![AccountingEntry {
                entry_type: AccountingEntryType::Credit,
                account: "non_staking_account".to_string(),
                account_type: AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: 1000,
                timestamp: 123456789,
                counterparty: "counterparty_1".to_string(),
                transfer_type: "Payment".to_string(),
            }],
            rhs: vec![],
        };

        // Publish event
        let event = Event {
            event_type: EventType::DoubleEntryTransaction,
            payload: sonic_rs::to_string(&non_staking_payload).unwrap(),
        };

        // Handle the non-staking event
        actor.handle_event(event).await;

        // Ensure no rows were inserted into the staking_ledger
        let rows = actor.client.query("SELECT * FROM staking_ledger", &[]).await.expect("Failed to query database");
        assert!(rows.is_empty(), "Non-staking ledger entries should not be processed");
    }

    #[tokio::test]
    async fn test_height_is_published() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = StakingLedgerActor::new(Arc::clone(&shared_publisher), &None).await;

        // Create a staking ledger event payload
        let staking_payload = DoubleEntryRecordPayload {
            height: 20,
            state_hash: "staking_state_hash".to_string(),
            ledger_destination: LedgerDestination::StakingLedger, // Staking destination
            lhs: vec![AccountingEntry {
                entry_type: AccountingEntryType::Debit,
                account: "lhs_staking_account".to_string(),
                account_type: AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: 500,
                timestamp: 123456789,
                counterparty: "rhs_staking_account".to_string(),
                transfer_type: "StakeDelegation".to_string(),
            }],
            rhs: vec![AccountingEntry {
                entry_type: AccountingEntryType::Credit,
                account: "rhs_staking_account".to_string(),
                account_type: AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: 500,
                timestamp: 123456789,
                counterparty: "lhs_staking_account".to_string(),
                transfer_type: "StakeDelegation".to_string(),
            }],
        };

        // Publish event
        let event = Event {
            event_type: EventType::DoubleEntryTransaction,
            payload: sonic_rs::to_string(&staking_payload).unwrap(),
        };

        // Subscribe to publisher
        let mut receiver = shared_publisher.subscribe();

        // Handle the staking event
        actor.handle_event(event).await;

        // Verify that the height was published
        let published_event = receiver.recv().await.expect("No event was published");
        assert_eq!(published_event.event_type, EventType::ActorHeight);

        let payload: ActorHeightPayload = sonic_rs::from_str(&published_event.payload).expect("Failed to deserialize payload");
        assert_eq!(payload.height, staking_payload.height, "Published height should match the payload height");
    }
}
