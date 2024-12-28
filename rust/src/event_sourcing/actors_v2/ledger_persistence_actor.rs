use crate::{
    constants::POSTGRES_CONNECTION_STRING,
    event_sourcing::{
        actor_dag::{ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
        events::EventType,
        managed_table::ManagedTable,
        payloads::{AccountingEntry, AccountingEntryType, DoubleEntryRecordPayload, LedgerDestination},
    },
};
use async_trait::async_trait;
use itertools::Itertools;
use log::error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::{types::ToSql, NoTls};

pub struct LedgerPersistenceActor;

const LEDGER_TABLE_KEY: &str = "ledger_table";

impl LedgerPersistenceActor {
    async fn bulk_insert(table: &mut ManagedTable, lhs: &[AccountingEntry], rhs: &[AccountingEntry], record_height: i64, record_state_hash: &str) {
        // 1) Interleave the slices into a single `Vec<AccountingEntry>`
        let combined_entries: Vec<AccountingEntry> = lhs
            .iter()
            .interleave(rhs.iter()) // or .interleave_shortest() if that's your preference
            .cloned() // assuming AccountingEntry is Clone
            .collect();

        // 2) Convert each AccountingEntry into a tuple of your desired columns (address, address_type, balance_delta, counterparty, transfer_type, height,
        //    state_hash, timestamp, ...)
        let values: Vec<(String, String, i64, String, String, i64, String, i64)> = combined_entries
            .iter()
            .map(|entry| {
                let balance_delta: i64 = match entry.entry_type {
                    AccountingEntryType::Credit => entry.amount_nanomina as i64,
                    AccountingEntryType::Debit => -(entry.amount_nanomina as i64),
                };

                // Return a tuple representing the row data
                (
                    entry.account.clone(),          // address
                    entry.account_type.to_string(), // address_type
                    balance_delta,                  // balance_delta
                    entry.counterparty.clone(),     // counterparty
                    entry.transfer_type.clone(),    // transfer_type
                    record_height,                  // height
                    record_state_hash.to_string(),  // state_hash
                    entry.timestamp as i64,         // timestamp
                )
            })
            .collect();

        // 3) Build the rows: a `Vec<Vec<&(dyn ToSql + Sync)>>`. Similar to your "other example" code snippet:
        let rows: Vec<Vec<&(dyn ToSql + Sync)>> = values
            .iter()
            .map(
                |(address, address_type, balance_delta, counterparty, transfer_type, height, state_hash, timestamp)| {
                    vec![
                        address as &(dyn ToSql + Sync),
                        address_type as &(dyn ToSql + Sync),
                        balance_delta as &(dyn ToSql + Sync),
                        counterparty as &(dyn ToSql + Sync),
                        transfer_type as &(dyn ToSql + Sync),
                        height as &(dyn ToSql + Sync),
                        state_hash as &(dyn ToSql + Sync),
                        timestamp as &(dyn ToSql + Sync),
                    ]
                },
            )
            .collect();

        // 4) Call bulk_insert once
        if let Err(e) = table.bulk_insert(&rows).await {
            error!("bulk_insert error: {}", e);
        }
    }
}

#[async_trait]
impl ActorFactory for LedgerPersistenceActor {
    async fn create_actor() -> ActorNode {
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Unable to connect to database");
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("connection error: {}", e);
            }
        });

        // 3) Build the ManagedTable (blocking)
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
            .build(&None)
            .await
            .expect("Failed building ledger table in LedgerPersistenceActor");

        // 4) Put the table in the ActorStore
        let mut store = ActorStore::new();
        store.insert(LEDGER_TABLE_KEY, table);

        // 5) Build the actor node with a processor
        ActorNodeBuilder::new()
            .with_state(store)
            .with_processor(|event, actor_store: Arc<Mutex<ActorStore>>, _requeue| {
                Box::pin(async move {
                    if event.event_type == EventType::DoubleEntryTransaction {
                        //  a) parse
                        let record: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).expect("Failed to parse DoubleEntryRecordPayload");

                        //  b) check ledger destination
                        if record.ledger_destination != LedgerDestination::BlockchainLedger {
                            return None;
                        }

                        //  c) get our table out of the store
                        let mut store_locked = actor_store.lock().await;
                        let mut table = store_locked
                            .remove::<ManagedTable>(LEDGER_TABLE_KEY)
                            .expect("Ledger table missing from ActorStore");

                        Self::bulk_insert(&mut table, &record.lhs, &record.rhs, record.height as i64, &record.state_hash).await;

                        store_locked.insert(LEDGER_TABLE_KEY, table);
                    }
                    None
                })
            })
            .build()
    }
}

#[cfg(test)]
mod ledger_persistence_actor_tests_v2 {
    use super::LedgerPersistenceActor;
    use crate::{
        constants::POSTGRES_CONNECTION_STRING,
        event_sourcing::{
            actor_dag::{ActorDAG, ActorFactory},
            events::EventType,
            payloads::{AccountingEntry, AccountingEntryType, DoubleEntryRecordPayload, LedgerDestination},
        },
    };
    use log::info;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tokio_postgres::NoTls;

    // A helper that queries rows from `blockchain_ledger` for test verification
    async fn fetch_ledger_rows() -> Vec<(String, i64)> {
        // Connect to the same DB
        let (client, conn) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Failed to connect in test");

        // Spawn the connection handle
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                eprintln!("Test connection error: {e}");
            }
        });

        // For demonstration, we'll just grab (address, balance_delta) to confirm row count
        let rows = client
            .query("SELECT address, balance_delta FROM blockchain_ledger ORDER BY entry_id ASC", &[])
            .await
            .expect("Failed to query ledger rows");

        rows.into_iter()
            .map(|row| {
                let address: String = row.get("address");
                let balance_delta: i64 = row.get("balance_delta");
                (address, balance_delta)
            })
            .collect()
    }

    #[tokio::test]
    async fn test_ledger_persistence_actor_interleave_inserts() {
        let mut dag = ActorDAG::new();

        // 2) Create the LedgerPersistenceActor node, set it as root
        let ledger_actor = LedgerPersistenceActor::create_actor().await;
        let ledger_sender = dag.set_root(ledger_actor);

        // 3) Spawn the DAG
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 4) Construct a DoubleEntryRecordPayload with mismatched `lhs` and `rhs`. E.g., 2 LHS, 3 RHS => total 5 entries when interleaved.
        let lhs_entries = vec![
            AccountingEntry {
                entry_type: AccountingEntryType::Debit,
                account: "lhs_debit_1".to_string(),
                account_type: crate::event_sourcing::payloads::AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: 10000,
                timestamp: 111_222_333,
                counterparty: "CP_lhs1".to_string(),
                transfer_type: "Transfer_lhs1".to_string(),
            },
            AccountingEntry {
                entry_type: AccountingEntryType::Credit,
                account: "lhs_credit_1".to_string(),
                account_type: crate::event_sourcing::payloads::AccountingEntryAccountType::VirtualAddess,
                amount_nanomina: 2000,
                timestamp: 111_222_334,
                counterparty: "CP_lhs2".to_string(),
                transfer_type: "Transfer_lhs2".to_string(),
            },
        ];
        let rhs_entries = vec![
            AccountingEntry {
                entry_type: AccountingEntryType::Credit,
                account: "rhs_credit_1".to_string(),
                account_type: crate::event_sourcing::payloads::AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: 3000,
                timestamp: 111_222_335,
                counterparty: "CP_rhs1".to_string(),
                transfer_type: "Transfer_rhs1".to_string(),
            },
            AccountingEntry {
                entry_type: AccountingEntryType::Debit,
                account: "rhs_debit_1".to_string(),
                account_type: crate::event_sourcing::payloads::AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: 4000,
                timestamp: 111_222_336,
                counterparty: "CP_rhs2".to_string(),
                transfer_type: "Transfer_rhs2".to_string(),
            },
            AccountingEntry {
                entry_type: AccountingEntryType::Credit,
                account: "rhs_credit_2".to_string(),
                account_type: crate::event_sourcing::payloads::AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: 5000,
                timestamp: 111_222_337,
                counterparty: "CP_rhs3".to_string(),
                transfer_type: "Transfer_rhs3".to_string(),
            },
        ];

        let record = DoubleEntryRecordPayload {
            height: 777,
            state_hash: "test_state_hash_interleave".to_string(),
            ledger_destination: LedgerDestination::BlockchainLedger,
            lhs: lhs_entries,
            rhs: rhs_entries,
        };

        // 5) Send the event
        ledger_sender
            .send(crate::event_sourcing::events::Event {
                event_type: EventType::DoubleEntryTransaction,
                payload: sonic_rs::to_string(&record).unwrap(),
            })
            .await
            .expect("Failed to send double entry record event");

        // 6) Wait a bit for the DAG to process
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // 7) Query the database
        let inserted_rows = fetch_ledger_rows().await;

        // Just log them for demonstration
        for (addr, bal) in &inserted_rows {
            info!("Row => address={}, balance_delta={}", addr, bal);
        }

        assert_eq!(inserted_rows.len(), 5, "Expected 5 rows");

        assert_eq!(inserted_rows[0], ("lhs_debit_1".to_string(), -10000));
        assert_eq!(inserted_rows[1], ("rhs_credit_1".to_string(), 3000));
        assert_eq!(inserted_rows[2], ("lhs_credit_1".to_string(), 2000));
        assert_eq!(inserted_rows[3], ("rhs_debit_1".to_string(), -4000));
        assert_eq!(inserted_rows[4], ("rhs_credit_2".to_string(), 5000));
    }
}
