use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    constants::POSTGRES_CONNECTION_STRING,
    event_sourcing::{
        managed_table::ManagedTable,
        payloads::{AccountingEntry, AccountingEntryType, ActorHeightPayload, DoubleEntryRecordPayload, LedgerDestination},
    },
};
use anyhow::Result;
use async_trait::async_trait;
use futures::lock::Mutex;
use log::error;
use std::sync::{atomic::AtomicUsize, Arc};
use tokio_postgres::NoTls;

pub struct LedgerActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub database_inserts: AtomicUsize,
    pub table: Arc<Mutex<ManagedTable>>,
}

impl LedgerActor {
    pub async fn new(shared_publisher: Arc<SharedPublisher>, root_node: &Option<(u64, String)>) -> Self {
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Unable to connect to database");
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("connection error: {}", e);
            }
        });

        let table = ManagedTable::builder(client)
            .name("blockchain_ledger")
            .add_column("address TEXT NOT NULL")
            .add_column("address_type TEXT NOT NULL")
            .add_column("balance_delta BIGINT NOT NULL")
            .add_column("counterparty TEXT NOT NULL")
            .add_column("transfer_type TEXT NOT NULL")
            .add_column("height BIGINT NOT NULL")
            .add_column("state_hash TEXT NOT NULL")
            .add_column("timestamp BIGINT NOT NULL")
            .build(root_node)
            .await
            .unwrap();

        if let Err(e) = table
            .get_client()
            .execute(
                "CREATE OR REPLACE VIEW account_summary AS
                SELECT
                    address,
                    address_type,
                    SUM(balance_delta) AS balance,
                    MAX(height) AS latest_height
                FROM
                    blockchain_ledger
                GROUP BY
                    address, address_type
                ORDER BY
                    latest_height DESC;",
                &[],
            )
            .await
        {
            error!("Unable to create account_summary table {:?}", e);
        }
        Self {
            id: "LedgerActor".to_string(),
            shared_publisher,
            table: Arc::new(Mutex::new(table)),
            database_inserts: AtomicUsize::new(0),
        }
    }

    async fn log(&self, table: &ManagedTable, payload: &AccountingEntry, height: &i64, state_hash: &str, timestamp: &i64) -> Result<u64, &'static str> {
        let balance_delta: i64 = match payload.entry_type {
            AccountingEntryType::Credit => payload.amount_nanomina as i64,
            AccountingEntryType::Debit => -(payload.amount_nanomina as i64),
        };

        match table
            .insert(&[
                &payload.account,
                &payload.account_type.to_string(),
                &balance_delta,
                &payload.counterparty.to_string(),
                &payload.transfer_type.to_string(),
                height,
                &state_hash,
                timestamp,
            ])
            .await
        {
            Err(e) => {
                let msg = e.to_string();
                error!("{}", msg);
                Err("unable to upsert into blockchain_ledger table")
            }
            Ok(affected_rows) => Ok(affected_rows),
        }
    }
}

#[async_trait]
impl Actor for LedgerActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.database_inserts
    }

    async fn handle_event(&self, event: Event) {
        if event.event_type == EventType::DoubleEntryTransaction {
            let event_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
            if event_payload.ledger_destination != LedgerDestination::BlockchainLedger {
                return;
            }
            let table = self.table.lock().await;
            for accounting_entry in event_payload.lhs.iter().chain(event_payload.rhs.iter()) {
                match self
                    .log(
                        &table,
                        accounting_entry,
                        &(event_payload.height as i64),
                        &event_payload.state_hash,
                        &(accounting_entry.timestamp as i64),
                    )
                    .await
                {
                    Ok(affected_rows) => {
                        assert_eq!(affected_rows, 1);
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
                        panic!("{:?}", e);
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
mod blockchain_ledger_actor_tests {
    use super::*;
    use crate::event_sourcing::{
        events::{Event, EventType},
        payloads::{AccountingEntry, AccountingEntryAccountType, AccountingEntryType, DoubleEntryRecordPayload, LedgerDestination},
    };
    // use serial_test::serial;
    use std::sync::Arc;

    #[tokio::test]
    // #[serial]
    async fn test_db_update_inserts_new_entry() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = LedgerActor::new(shared_publisher, &None).await;

        // Accounting entry payload
        let accounting_entry = AccountingEntry {
            entry_type: AccountingEntryType::Credit,
            account: "test_address2".to_string(),
            account_type: AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: 1000,
            timestamp: 123456789,
            counterparty: "counterparty_1".to_string(),
            transfer_type: "transfer_type_1".to_string(),
        };

        let height: i64 = 10;
        let state_hash = "test_state_hash";
        let table = actor.table.lock().await;

        // Perform database update
        let result = actor
            .log(&table, &accounting_entry, &height, state_hash, &(accounting_entry.timestamp as i64))
            .await;

        // Assert successful insertion
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        // Query the database directly to validate the record

        let rows = table
            .get_client()
            .query("SELECT * FROM blockchain_ledger WHERE address = $1", &[&accounting_entry.account])
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
    // #[serial]
    async fn test_db_update_updates_existing_entry() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = LedgerActor::new(shared_publisher, &None).await;

        // Initial accounting entry
        let accounting_entry = AccountingEntry {
            entry_type: AccountingEntryType::Credit,
            account: "test_address".to_string(),
            account_type: AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: 1000,
            timestamp: 123456789,
            counterparty: "counterparty_1".to_string(),
            transfer_type: "transfer_type_1".to_string(),
        };

        let height: i64 = 10;
        let state_hash = "test_state_hash";

        let table = actor.table.lock().await;

        actor
            .log(&table, &accounting_entry, &height, state_hash, &(accounting_entry.timestamp as i64))
            .await
            .unwrap();

        // Update the same entry with new data (append new log entry)
        let updated_entry = AccountingEntry {
            entry_type: AccountingEntryType::Debit,
            account: "test_address".to_string(),
            account_type: AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: 500,
            timestamp: 123456790,
            counterparty: "counterparty_1".to_string(),
            transfer_type: "transfer_type_1".to_string(),
        };

        let updated_height: i64 = 11; // Use a new height for the append-only log
        let updated_state_hash = "updated_state_hash";

        let result = actor
            .log(&table, &updated_entry, &updated_height, updated_state_hash, &(updated_entry.timestamp as i64))
            .await;

        // Assert successful append
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        // Query the database to validate the appended records

        let rows = table
            .get_client()
            .query(
                "SELECT * FROM blockchain_ledger WHERE address = $1 ORDER BY timestamp ASC",
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
    // #[serial]
    async fn test_handle_event_processes_double_entry_transaction() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = LedgerActor::new(Arc::clone(&shared_publisher), &None).await;

        // Event payload
        let double_entry_payload = DoubleEntryRecordPayload {
            height: 9, // Matches modulo 3
            state_hash: "test_state_hash".to_string(),
            ledger_destination: LedgerDestination::BlockchainLedger,
            lhs: vec![AccountingEntry {
                entry_type: AccountingEntryType::Debit,
                account: "lhs_account".to_string(),
                account_type: AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: 1000,
                timestamp: 123456789,
                counterparty: "counterparty_1".to_string(),
                transfer_type: "transfer_type_1".to_string(),
            }],
            rhs: vec![AccountingEntry {
                entry_type: AccountingEntryType::Credit,
                account: "rhs_account".to_string(),
                account_type: AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: 1000,
                timestamp: 123456789,
                counterparty: "counterparty_1".to_string(),
                transfer_type: "transfer_type_1".to_string(),
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
            let table = actor.table.lock().await;
            let rows = table
                .get_client()
                .query("SELECT * FROM blockchain_ledger WHERE address = $1", &[&account])
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

    #[tokio::test]
    async fn test_publishes_heights() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = LedgerActor::new(Arc::clone(&shared_publisher), &None).await;

        // Define a double-entry transaction payload
        let double_entry_payload = DoubleEntryRecordPayload {
            height: 15,
            state_hash: "test_state_hash".to_string(),
            ledger_destination: LedgerDestination::BlockchainLedger,
            lhs: vec![AccountingEntry {
                entry_type: AccountingEntryType::Debit,
                account: "lhs_account".to_string(),
                account_type: AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: 1000,
                timestamp: 123456789,
                counterparty: "counterparty_1".to_string(),
                transfer_type: "transfer_type_1".to_string(),
            }],
            rhs: vec![AccountingEntry {
                entry_type: AccountingEntryType::Credit,
                account: "rhs_account".to_string(),
                account_type: AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: 1000,
                timestamp: 123456789,
                counterparty: "counterparty_1".to_string(),
                transfer_type: "transfer_type_1".to_string(),
            }],
        };

        // Create an event with the payload
        let event = Event {
            event_type: EventType::DoubleEntryTransaction,
            payload: sonic_rs::to_string(&double_entry_payload).unwrap(),
        };

        let mut receiver = shared_publisher.subscribe();

        // Process the event
        actor.handle_event(event).await;

        // Verify that the height was published
        let published_event = receiver.recv().await.expect("No event was published");

        assert_eq!(published_event.event_type, EventType::ActorHeight);
        let payload: ActorHeightPayload = sonic_rs::from_str(&published_event.payload).expect("Failed to deserialize payload");
        assert_eq!(payload.height, double_entry_payload.height);

        println!("Published height: {}", payload.height);
    }

    #[tokio::test]
    async fn test_ledger_actor_discard_entries_at_or_above_root_node() {
        use crate::event_sourcing::payloads::{AccountingEntry, AccountingEntryAccountType, AccountingEntryType};
        use std::sync::Arc;

        // Step 1: Create a shared publisher and initialize the actor without a root_node to populate the ledger
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor_without_root = LedgerActor::new(Arc::clone(&shared_publisher), &None).await;

        // Add ledger entries with varying heights
        let ledger_entries = vec![
            (5, "hash_5"),   // Below the root height
            (10, "hash_10"), // At the root height
            (15, "hash_15"), // Above the root height
        ];

        let table = actor_without_root.table.lock().await;

        for (height, state_hash) in &ledger_entries {
            let accounting_entry = AccountingEntry {
                entry_type: AccountingEntryType::Credit,
                account: format!("account_{}", height),
                account_type: AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: 1000,
                timestamp: 123456789 + height,
                counterparty: "counterparty".to_string(),
                transfer_type: "transfer_type".to_string(),
            };

            actor_without_root
                .log(&table, &accounting_entry, &(*height as i64), state_hash, &(123456789_i64))
                .await
                .expect("Failed to log accounting entry");
        }

        // Verify that all entries were inserted
        let rows = table
            .get_client()
            .query("SELECT * FROM blockchain_ledger ORDER BY height ASC", &[])
            .await
            .expect("Failed to query blockchain_ledger");
        assert_eq!(rows.len(), ledger_entries.len(), "All entries should be present initially");

        // Step 2: Initialize the actor with a root_node and verify entries are discarded
        let root_node_height = 10;
        let root_node = Some((root_node_height, "hash_10".to_string()));
        let actor_with_root = LedgerActor::new(Arc::clone(&shared_publisher), &root_node).await;

        // Query the ledger after reinitialization
        let table = actor_with_root.table.lock().await;
        let rows_after_reinit = table
            .get_client()
            .query("SELECT * FROM blockchain_ledger ORDER BY height ASC", &[])
            .await
            .expect("Failed to query blockchain_ledger after reinitialization");

        // Verify that entries at or above the root_node_height are discarded
        assert_eq!(rows_after_reinit.len(), 1, "Only entries below the root node height should remain");

        // Verify the remaining entry is below the root node height
        let remaining_entry = &rows_after_reinit[0];
        assert_eq!(
            remaining_entry.get::<_, i64>("height"),
            5,
            "The only remaining entry should be below the root node height"
        );
        assert_eq!(
            remaining_entry.get::<_, String>("state_hash"),
            "hash_5",
            "The only remaining entry should have the correct state_hash"
        );
    }
}
