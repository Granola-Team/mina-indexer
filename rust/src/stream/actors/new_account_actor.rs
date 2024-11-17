use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    constants::POSTGRES_CONNECTION_STRING,
    stream::payloads::{AccountingEntryAccountType, BlockConfirmationPayload, DoubleEntryRecordPayload, NewAccountPayload},
};
use anyhow::Result;
use async_trait::async_trait;
use std::{
    collections::VecDeque,
    sync::{atomic::AtomicUsize, Arc},
};
use tokio::sync::Mutex;
use tokio_postgres::{error::SqlState, Client, NoTls};

pub struct NewAccountActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub database_inserts: AtomicUsize,
    pub client: Client,
    pub deep_canonical_block: Mutex<Option<BlockConfirmationPayload>>,
    pub transaction_queue: Mutex<VecDeque<DoubleEntryRecordPayload>>,
}

impl NewAccountActor {
    pub async fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        if let Ok((client, connection)) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls).await {
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {}", e);
                }
            });
            if let Err(e) = client.execute("DROP TABLE IF EXISTS account_tracking;", &[]).await {
                println!("Unable to drop account_tracking table {:?}", e);
            }
            if let Err(e) = client
                .execute(
                    "CREATE TABLE IF NOT EXISTS account_tracking (
                        account TEXT PRIMARY KEY,
                        height BIGINT NOT NULL
                    );",
                    &[],
                )
                .await
            {
                println!("Unable to create account_tracking table {:?}", e);
            }
            Self {
                id: "NewAccountActor".to_string(),
                shared_publisher,
                client,
                database_inserts: AtomicUsize::new(0),
                deep_canonical_block: Mutex::new(None),
                transaction_queue: Mutex::new(VecDeque::new()),
            }
        } else {
            panic!("Unable to establish connection to database")
        }
    }

    async fn insert_account(&self, account: &str, height: i64) -> Result<u64, &'static str> {
        let insert_query = r#"
            INSERT INTO account_tracking (account, height)
            VALUES ($1, $2)
        "#;

        match self.client.execute(insert_query, &[&account, &height]).await {
            Ok(affected_rows) => Ok(affected_rows),
            Err(e) => {
                if let Some(db_error) = e.as_db_error() {
                    if db_error.code() == &SqlState::UNIQUE_VIOLATION {
                        return Err("duplicate key violation");
                    }
                }
                println!("Database error: {:?}", e);
                Err("unable to insert into account_tracking table")
            }
        }
    }

    async fn process_transaction(&self, transaction: &DoubleEntryRecordPayload) {
        let block = self.deep_canonical_block.lock().await;
        if let Some(confirmed_block) = &*block {
            if transaction.height == confirmed_block.height && transaction.state_hash == confirmed_block.state_hash {
                for accounting_entry in transaction.lhs.iter().chain(transaction.rhs.iter()) {
                    let account = &accounting_entry.account;
                    if accounting_entry.account_type == AccountingEntryAccountType::BlockchainAddress {
                        match self.insert_account(account, transaction.height as i64).await {
                            Ok(affected_rows) => {
                                if affected_rows == 1 {
                                    // Publish NewAccount event
                                    let new_account_event = Event {
                                        event_type: EventType::NewAccount,
                                        payload: sonic_rs::to_string(&NewAccountPayload {
                                            height: transaction.height,
                                            state_hash: transaction.state_hash.to_string(),
                                            timestamp: accounting_entry.timestamp,
                                            account: account.to_string(),
                                        })
                                        .unwrap(),
                                    };
                                    self.publish(new_account_event);
                                    self.shared_publisher.incr_database_insert();
                                }
                            }
                            Err(e) => {
                                if e != "duplicate key violation" {
                                    eprintln!("Error inserting account: {:?}", e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    async fn process_queue(&self) {
        let mut queue = self.transaction_queue.lock().await;
        while let Some(transaction) = queue.pop_front() {
            self.process_transaction(&transaction).await;
        }
    }
}

#[async_trait]
impl Actor for NewAccountActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.database_inserts
    }

    async fn handle_event(&self, event: Event) {
        match event.event_type {
            EventType::BlockConfirmation => {
                let payload: BlockConfirmationPayload = sonic_rs::from_str(&event.payload).unwrap();

                if payload.confirmations >= 10 {
                    let mut block = self.deep_canonical_block.lock().await;
                    *block = Some(payload);
                    drop(block);

                    // Process the transaction queue
                    self.process_queue().await;
                }
            }
            EventType::DoubleEntryTransaction => {
                let payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
                let mut queue = self.transaction_queue.lock().await;
                queue.push_back(payload);
            }
            _ => {}
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }

    async fn report(&self) {
        let txn_queue = self.transaction_queue.lock().await;
        self.print_report("Transaction VecDeque", txn_queue.len());
        drop(txn_queue);
    }
}

#[cfg(test)]
mod new_account_actor_tests {
    use super::*;
    use crate::stream::{
        events::{Event, EventType},
        payloads::{AccountingEntry, AccountingEntryAccountType, AccountingEntryType, BlockConfirmationPayload, DoubleEntryRecordPayload, NewAccountPayload},
    };
    use std::sync::Arc;
    use tokio::time::timeout;

    async fn setup_actor() -> (Arc<NewAccountActor>, tokio::sync::broadcast::Receiver<Event>) {
        let shared_publisher = Arc::new(SharedPublisher::new(200));
        let actor = NewAccountActor::new(Arc::clone(&shared_publisher)).await;
        let receiver = shared_publisher.subscribe();
        (Arc::new(actor), receiver)
    }

    #[tokio::test]
    async fn test_handle_block_confirmation() {
        let (actor, _) = setup_actor().await;

        let payload = BlockConfirmationPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            confirmations: 10,
        };

        let event = Event {
            event_type: EventType::BlockConfirmation,
            payload: sonic_rs::to_string(&payload).unwrap(),
        };

        actor.handle_event(event).await;

        let deep_canonical_block = actor.deep_canonical_block.lock().await;
        assert!(deep_canonical_block.is_some());
        assert_eq!(deep_canonical_block.as_ref().unwrap().height, 1);
        assert_eq!(deep_canonical_block.as_ref().unwrap().state_hash, "hash_1");
    }

    #[tokio::test]
    async fn test_handle_double_entry_transaction() {
        let (actor, _) = setup_actor().await;

        let unique_account = "B62qtestaccount1".to_string();

        let payload = DoubleEntryRecordPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            lhs: vec![AccountingEntry {
                entry_type: AccountingEntryType::Debit,
                account: unique_account.clone(),
                account_type: AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: 100_000,
                timestamp: 123456789,
            }],
            rhs: vec![],
        };

        let event = Event {
            event_type: EventType::DoubleEntryTransaction,
            payload: sonic_rs::to_string(&payload).unwrap(),
        };

        actor.handle_event(event).await;

        let txn_queue = actor.transaction_queue.lock().await;
        assert_eq!(txn_queue.len(), 1);
        let queued_txn = txn_queue.front().unwrap();
        assert_eq!(queued_txn.height, 1);
        assert_eq!(queued_txn.state_hash, "hash_1");
    }

    #[tokio::test]
    async fn test_handle_event_with_queue_processing() {
        let (actor, mut receiver) = setup_actor().await;

        // Use a unique account for this test
        let unique_account = "B62qtestaccount2".to_string();

        // Publish a DoubleEntryTransaction event first
        let transaction_payload = DoubleEntryRecordPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            lhs: vec![AccountingEntry {
                entry_type: AccountingEntryType::Debit,
                account: unique_account.clone(),
                account_type: AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: 100_000,
                timestamp: 123456789,
            }],
            rhs: vec![],
        };

        let transaction_event = Event {
            event_type: EventType::DoubleEntryTransaction,
            payload: sonic_rs::to_string(&transaction_payload).unwrap(),
        };

        actor.handle_event(transaction_event).await;

        // Publish a BlockConfirmation event with 10 confirmations
        let confirmation_payload = BlockConfirmationPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            confirmations: 10,
        };

        let confirmation_event = Event {
            event_type: EventType::BlockConfirmation,
            payload: sonic_rs::to_string(&confirmation_payload).unwrap(),
        };

        actor.handle_event(confirmation_event).await;

        // Verify that the NewAccount event is published
        if let Ok(received_event) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            let event = received_event.unwrap();
            assert_eq!(event.event_type, EventType::NewAccount);

            let new_account_payload: NewAccountPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(new_account_payload.height, 1);
            assert_eq!(new_account_payload.account, unique_account);
        } else {
            panic!("Did not receive NewAccount event");
        }
    }

    #[tokio::test]
    async fn test_handle_event_with_non_matching_transaction() {
        let (actor, mut receiver) = setup_actor().await;

        let unique_account = "B62qtestaccount3".to_string();

        {
            let mut deep_canonical_block = actor.deep_canonical_block.lock().await;
            *deep_canonical_block = Some(BlockConfirmationPayload {
                height: 1,
                state_hash: "hash_1".to_string(),
                confirmations: 10,
            });
        }

        let transaction_payload = DoubleEntryRecordPayload {
            height: 2,
            state_hash: "hash_2".to_string(),
            lhs: vec![AccountingEntry {
                entry_type: AccountingEntryType::Debit,
                account: unique_account.clone(),
                account_type: AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: 100_000,
                timestamp: 123456789,
            }],
            rhs: vec![],
        };

        let transaction_event = Event {
            event_type: EventType::DoubleEntryTransaction,
            payload: sonic_rs::to_string(&transaction_payload).unwrap(),
        };

        actor.handle_event(transaction_event).await;

        let received_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(received_event.is_err(), "No NewAccount event should be published for non-matching blocks.");
    }

    #[tokio::test]
    async fn test_handle_event_does_not_publish_duplicate_account() {
        let (actor, mut receiver) = setup_actor().await;

        // Use a unique account for this test
        let unique_account = "B62qtestaccount3".to_string();

        // Manually insert the account into the database
        actor
            .client
            .execute("INSERT INTO account_tracking (account, height) VALUES ($1, $2)", &[&unique_account, &1_i64])
            .await
            .expect("Failed to insert account into the database");

        // Publish a DoubleEntryTransaction event
        let transaction_payload = DoubleEntryRecordPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            lhs: vec![AccountingEntry {
                entry_type: AccountingEntryType::Debit,
                account: unique_account.clone(),
                account_type: AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: 100_000,
                timestamp: 123456789,
            }],
            rhs: vec![],
        };

        let transaction_event = Event {
            event_type: EventType::DoubleEntryTransaction,
            payload: sonic_rs::to_string(&transaction_payload).unwrap(),
        };

        actor.handle_event(transaction_event).await;

        // Publish a BlockConfirmation event with 10 confirmations
        let confirmation_payload = BlockConfirmationPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            confirmations: 10,
        };

        let confirmation_event = Event {
            event_type: EventType::BlockConfirmation,
            payload: sonic_rs::to_string(&confirmation_payload).unwrap(),
        };

        actor.handle_event(confirmation_event).await;

        // Verify that no NewAccount event is published
        let received_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(
            received_event.is_err(),
            "No NewAccount event should be published for an already tracked account."
        );
    }
}
