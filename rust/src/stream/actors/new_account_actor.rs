use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    constants::POSTGRES_CONNECTION_STRING,
    stream::payloads::{AccountingEntryAccountType, DoubleEntryRecordPayload, NewAccountPayload},
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};
use tokio_postgres::{error::SqlState, Client, NoTls};

pub struct NewAccountActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub database_inserts: AtomicUsize,
    pub client: Client,
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
        if event.event_type == EventType::DoubleEntryTransaction {
            let event_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();

            for accounting_entry in event_payload.lhs.iter().chain(event_payload.rhs.iter()) {
                let account = &accounting_entry.account;
                if accounting_entry.account_type == AccountingEntryAccountType::BlockchainAddress {
                    match self.insert_account(account, event_payload.height as i64).await {
                        Ok(affected_rows) => {
                            if affected_rows == 1 {
                                // Publish NewAccount event
                                let new_account_event = Event {
                                    event_type: EventType::NewAccount,
                                    payload: sonic_rs::to_string(&NewAccountPayload {
                                        height: event_payload.height,
                                        state_hash: event_payload.state_hash.to_string(),
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
                                panic!("Error inserting account: {:?}", e);
                            }
                            // Duplicate key, do nothing
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
mod new_account_actor_tests {
    use super::*;
    use crate::stream::{
        events::{Event, EventType},
        payloads::{AccountingEntry, AccountingEntryAccountType, AccountingEntryType, DoubleEntryRecordPayload, NewAccountPayload},
    };
    use std::sync::Arc;
    use tokio::time::timeout;

    async fn setup_actor() -> (Arc<NewAccountActor>, tokio::sync::broadcast::Receiver<Event>) {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = NewAccountActor::new(Arc::clone(&shared_publisher)).await;
        let receiver = shared_publisher.subscribe();
        (Arc::new(actor), receiver)
    }

    #[tokio::test]
    async fn test_insert_new_account() {
        let (actor, mut receiver) = setup_actor().await;

        let payload = DoubleEntryRecordPayload {
            height: 1,
            state_hash: "state_hash".to_string(),
            lhs: vec![AccountingEntry {
                entry_type: AccountingEntryType::Debit,
                account: "B62qnewaccount1".to_string(),
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

        if let Ok(received_event) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            let event = received_event.unwrap();
            assert_eq!(event.event_type, EventType::NewAccount);
            let new_account_payload: NewAccountPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(new_account_payload.height, payload.height);
            assert_eq!(new_account_payload.account, "B62qnewaccount1".to_string());
        } else {
            panic!("Did not receive NewAccount event");
        }

        assert_eq!(actor.actor_outputs().load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_duplicate_account_does_not_publish_event() {
        let (actor, mut receiver) = setup_actor().await;

        let payload = DoubleEntryRecordPayload {
            height: 1,
            state_hash: "state_hash".to_string(),
            lhs: vec![AccountingEntry {
                entry_type: AccountingEntryType::Debit,
                account: "B62qduplicateaccount".to_string(),
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

        // First insert
        actor.handle_event(event.clone()).await;

        // Second insert with duplicate account
        actor.handle_event(event).await;

        // Only one NewAccount event should be published
        let received_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(received_event.is_ok());

        let received_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(received_event.is_err(), "Duplicate account should not publish a NewAccount event");

        assert_eq!(actor.actor_outputs().load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
