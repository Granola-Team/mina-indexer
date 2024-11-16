use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    constants::POSTGRES_CONNECTION_STRING,
    stream::payloads::{AccountingEntry, AccountingEntryType, DoubleEntryRecordPayload},
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};
use tokio_postgres::{Client, NoTls};

pub struct AccountSummaryPersistenceActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub database_inserts: AtomicUsize,
    pub modulo_3: u64,
    pub client: Client,
}

impl AccountSummaryPersistenceActor {
    pub async fn new(shared_publisher: Arc<SharedPublisher>, modulo_3: u64) -> Self {
        if let Ok((client, connection)) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls).await {
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {}", e);
                }
            });
            if let Err(e) = client.execute("DROP TABLE IF EXISTS accounts_log CASCADE;", &[]).await {
                println!("Unable to drop accounts_log table {:?}", e);
            }
            if let Err(e) = client
                .execute(
                    "CREATE TABLE IF NOT EXISTS accounts_log (
                        address TEXT NOT NULL,
                        address_type TEXT NOT NULL,
                        balance_delta BIGINT NOT NULL,
                        height BIGINT NOT NULL,
                        state_hash TEXT NOT NULL,
                        timestamp BIGINT NOT NULL,
                        PRIMARY KEY (address, height, state_hash, timestamp)
                    );",
                    &[],
                )
                .await
            {
                println!("Unable to create accounts_log table {:?}", e);
            }
            if let Err(e) = client
                .execute(
                    "CREATE OR REPLACE VIEW account_balance_summary AS
                    SELECT
                        a.address,
                        a.address_type,
                        SUM(a.balance_delta) AS total_balance,
                        MAX(a.height) AS latest_height,
                        (SELECT state_hash FROM accounts_log WHERE address = a.address AND height = MAX(a.height) ORDER BY timestamp DESC LIMIT 1) AS latest_state_hash,
                        (SELECT timestamp FROM accounts_log WHERE address = a.address AND height = MAX(a.height) ORDER BY timestamp DESC LIMIT 1) AS latest_timestamp
                    FROM
                        accounts_log a
                    GROUP BY
                        a.address, a.address_type
                    ORDER BY
                        a.address;",
                    &[],
                )
                .await
            {
                println!("Unable to create accounts_log table {:?}", e);
            }
            Self {
                id: "AccountSummaryPersistenceActor".to_string(),
                shared_publisher,
                client,
                modulo_3,
                database_inserts: AtomicUsize::new(0),
            }
        } else {
            panic!("Unable to establish connection to database")
        }
    }

    async fn db_update(&self, payload: &AccountingEntry, height: &i64, state_hash: &str, timestamp: &i64) -> Result<u64, &'static str> {
        let upsert_query = r#"
            INSERT INTO accounts_log (address, address_type, balance_delta, height, state_hash, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (address, height, state_hash, timestamp)
            DO UPDATE SET balance_delta = $3;
        "#;

        let amount: i64 = match payload.entry_type {
            AccountingEntryType::Credit => payload.amount_nanomina as i64,
            AccountingEntryType::Debit => -(payload.amount_nanomina as i64),
        };

        match self
            .client
            .execute(
                upsert_query,
                &[&payload.account, &payload.account_type.to_string(), &amount, height, &state_hash, timestamp],
            )
            .await
        {
            Err(e) => {
                let msg = e.to_string();
                println!("{}", msg);
                Err("unable to upsert into accounts_log table")
            }
            Ok(affected_rows) => Ok(affected_rows),
        }
    }
}

#[async_trait]
impl Actor for AccountSummaryPersistenceActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.database_inserts
    }

    async fn handle_event(&self, event: Event) {
        if event.event_type == EventType::DoubleEntryTransaction {
            let event_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
            if event_payload.height % 3 == self.modulo_3 {
                for accounting_entry in event_payload.lhs.iter().chain(event_payload.rhs.iter()) {
                    match self
                        .db_update(
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
                            self.actor_outputs().fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        }
                        Err(e) => {
                            panic!("{:?}", e);
                        }
                    }
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
mod account_summary_persistence_actor_tests {
    use super::*;
    use crate::stream::{
        events::{Event, EventType},
        payloads::{AccountingEntry, AccountingEntryAccountType, AccountingEntryType, DoubleEntryRecordPayload},
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn test_db_update_inserts_new_entry() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = AccountSummaryPersistenceActor::new(shared_publisher, 0).await;

        // Accounting entry payload
        let accounting_entry = AccountingEntry {
            entry_type: AccountingEntryType::Credit,
            account: "test_address2".to_string(),
            account_type: AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: 1000,
            timestamp: 123456789,
        };

        let height: i64 = 10;
        let state_hash = "test_state_hash";

        // Perform database update
        let result = actor
            .db_update(&accounting_entry, &height, state_hash, &(accounting_entry.timestamp as i64))
            .await;

        // Assert successful insertion
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        // Query the database directly to validate the record
        let rows = actor
            .client
            .query("SELECT * FROM accounts_log WHERE address = $1", &[&accounting_entry.account])
            .await
            .expect("Failed to query database");
        assert_eq!(rows.len(), 1);

        let row = &rows[0];
        assert_eq!(row.get::<_, String>("address"), accounting_entry.account);
        assert_eq!(row.get::<_, String>("address_type"), "BlockchainAddress");
        assert_eq!(row.get::<_, i64>("balance_delta"), 1000);
        assert_eq!(row.get::<_, i64>("height"), height);
        assert_eq!(row.get::<_, String>("state_hash"), state_hash);
        assert_eq!(row.get::<_, i64>("timestamp"), 123456789);
    }

    #[tokio::test]
    async fn test_db_update_updates_existing_entry() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = AccountSummaryPersistenceActor::new(shared_publisher, 0).await;

        // Initial accounting entry
        let accounting_entry = AccountingEntry {
            entry_type: AccountingEntryType::Credit,
            account: "test_address".to_string(),
            account_type: AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: 1000,
            timestamp: 123456789,
        };

        let height: i64 = 10;
        let state_hash = "test_state_hash";

        actor
            .db_update(&accounting_entry, &height, state_hash, &(accounting_entry.timestamp as i64))
            .await
            .unwrap();

        // Update the same entry with new data (append new log entry)
        let updated_entry = AccountingEntry {
            entry_type: AccountingEntryType::Debit,
            account: "test_address".to_string(),
            account_type: AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: 500,
            timestamp: 123456790,
        };

        let updated_height: i64 = 11; // Use a new height for the append-only log
        let updated_state_hash = "updated_state_hash";

        let result = actor
            .db_update(&updated_entry, &updated_height, updated_state_hash, &(updated_entry.timestamp as i64))
            .await;

        // Assert successful append
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        // Query the database to validate the appended records
        let rows = actor
            .client
            .query(
                "SELECT * FROM accounts_log WHERE address = $1 ORDER BY timestamp ASC",
                &[&updated_entry.account],
            )
            .await
            .expect("Failed to query database");

        // There should now be two entries for the same address
        assert_eq!(rows.len(), 2);

        // Validate the first entry (initial log)
        let first_row = &rows[0];
        assert_eq!(first_row.get::<_, i64>("balance_delta"), 1000);
        assert_eq!(first_row.get::<_, i64>("height"), height);
        assert_eq!(first_row.get::<_, String>("state_hash"), state_hash);
        assert_eq!(first_row.get::<_, i64>("timestamp"), 123456789);

        // Validate the second entry (new log entry)
        let second_row = &rows[1];
        assert_eq!(second_row.get::<_, i64>("balance_delta"), -500);
        assert_eq!(second_row.get::<_, i64>("height"), updated_height);
        assert_eq!(second_row.get::<_, String>("state_hash"), updated_state_hash);
        assert_eq!(second_row.get::<_, i64>("timestamp"), 123456790);
    }

    #[tokio::test]
    async fn test_handle_event_processes_double_entry_transaction() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = AccountSummaryPersistenceActor::new(Arc::clone(&shared_publisher), 0).await;

        // Event payload
        let double_entry_payload = DoubleEntryRecordPayload {
            height: 9, // Matches modulo 3
            state_hash: "test_state_hash".to_string(),
            lhs: vec![AccountingEntry {
                entry_type: AccountingEntryType::Debit,
                account: "lhs_account".to_string(),
                account_type: AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: 1000,
                timestamp: 123456789,
            }],
            rhs: vec![AccountingEntry {
                entry_type: AccountingEntryType::Credit,
                account: "rhs_account".to_string(),
                account_type: AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: 1000,
                timestamp: 123456789,
            }],
        };

        // Publish event and process it
        let event = Event {
            event_type: EventType::DoubleEntryTransaction,
            payload: sonic_rs::to_string(&double_entry_payload).unwrap(),
        };
        actor.handle_event(event).await;

        // Query database to validate records
        for account in ["lhs_account", "rhs_account"] {
            let rows = actor
                .client
                .query("SELECT * FROM accounts_log WHERE address = $1", &[&account])
                .await
                .expect("Failed to query database");
            assert!(!rows.is_empty(), "No records found for account: {}", account);

            for row in rows {
                // Validate that records were inserted as expected
                assert_eq!(row.get::<_, String>("address"), account);
                assert_eq!(row.get::<_, i64>("height"), double_entry_payload.height as i64);
                assert_eq!(row.get::<_, String>("state_hash"), double_entry_payload.state_hash);
                assert_eq!(row.get::<_, i64>("timestamp"), 123456789);

                // Validate the balance_delta based on the entry type
                let balance_delta = row.get::<_, i64>("balance_delta");
                if account == "lhs_account" {
                    assert_eq!(balance_delta, -1000); // Debit
                } else if account == "rhs_account" {
                    assert_eq!(balance_delta, 1000); // Credit
                }
            }
        }
    }
}
