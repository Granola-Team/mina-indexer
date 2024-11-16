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
    pub client: Client,
}

impl AccountSummaryPersistenceActor {
    pub async fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        if let Ok((client, connection)) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls).await {
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {}", e);
                }
            });
            if let Err(e) = client.execute("DROP TABLE IF EXISTS accounts;", &[]).await {
                println!("Unable to drop accounts table {:?}", e);
            }
            if let Err(e) = client
                .execute(
                    "CREATE TABLE IF NOT EXISTS accounts (
                        address TEXT PRIMARY KEY,
                        address_type TEXT NOT NULL,
                        balance_nanomina BIGINT NOT NULL
                    );",
                    &[],
                )
                .await
            {
                println!("Unable to create accounts table {:?}", e);
            }
            Self {
                id: "AccountSummaryPersistenceActor".to_string(),
                shared_publisher,
                client,
                database_inserts: AtomicUsize::new(0),
            }
        } else {
            panic!("Unable to establish connection to database")
        }
    }

    async fn db_update(&self, payload: &AccountingEntry) -> Result<u64, &'static str> {
        let upsert_query = r#"
            INSERT INTO accounts (address, address_type, balance_nanomina)
            VALUES ($1, $2, $3)
            ON CONFLICT (address)
            DO UPDATE SET balance_nanomina = accounts.balance_nanomina + $3,
                address_type = $2
        "#;

        let amount: i64 = match payload.entry_type {
            AccountingEntryType::Credit => payload.amount_nanomina as i64,
            AccountingEntryType::Debit => -(payload.amount_nanomina as i64),
        };
        match self
            .client
            .execute(upsert_query, &[&payload.account, &payload.account_type.to_string(), &amount])
            .await
        {
            Err(e) => {
                let msg = e.to_string();
                println!("{}", msg);
                Err("unable to upsert into accounts table")
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
            for accounting_entry in event_payload.lhs.iter().chain(event_payload.rhs.iter()) {
                match self.db_update(accounting_entry).await {
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

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}
