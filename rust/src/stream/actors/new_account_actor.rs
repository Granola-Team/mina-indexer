use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    blockchain_tree::Height,
    constants::POSTGRES_CONNECTION_STRING,
    stream::payloads::{BlockConfirmationPayload, MainnetBlockPayload, NewAccountPayload},
};
use async_trait::async_trait;
use futures::lock::Mutex;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc},
};
use tokio_postgres::{Client, NoTls};

pub struct NewAccountActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
    pub database_inserts: AtomicUsize,
    pub mainnet_blocks: Arc<Mutex<HashMap<Height, Vec<MainnetBlockPayload>>>>,
    pub client: Client,
}

impl NewAccountActor {
    pub async fn new(shared_publisher: Arc<SharedPublisher>, preserve_existing_data: bool) -> Self {
        if let Ok((client, connection)) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls).await {
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {}", e);
                }
            });
            if !preserve_existing_data {
                if let Err(e) = client.execute("DROP TABLE IF EXISTS discovered_accounts;", &[]).await {
                    println!("Unable to drop user_commands table {:?}", e);
                }
            }
            if let Err(e) = client
                .execute(
                    "CREATE TABLE IF NOT EXISTS discovered_accounts (
                        account TEXT PRIMARY KEY NOT NULL
                    );
                    ",
                    &[],
                )
                .await
            {
                println!("Unable to create discovered_accounts table {:?}", e);
            }
            Self {
                id: "NewAccountActor".to_string(),
                shared_publisher,
                client,
                mainnet_blocks: Arc::new(Mutex::new(HashMap::new())),
                events_published: AtomicUsize::new(0),
                database_inserts: AtomicUsize::new(0),
            }
        } else {
            panic!("Unable to establish connection to database")
        }
    }
}
#[async_trait]
impl Actor for NewAccountActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.events_published
    }

    async fn handle_event(&self, event: Event) {
        match event.event_type {
            EventType::PreExistingAccount => {
                let account: String = event.payload.to_string();
                // Insert the account into the `discovered_accounts` table
                let insert_query = "INSERT INTO discovered_accounts (account) VALUES ($1) ON CONFLICT DO NOTHING";
                if let Err(e) = self.client.execute(insert_query, &[&account]).await {
                    eprintln!("Failed to insert account {} into database: {:?}", account, e);
                } else {
                    self.database_inserts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
            }
            EventType::MainnetBlock => {
                let block: MainnetBlockPayload = sonic_rs::from_str(&event.payload).unwrap();
                let mut mainnet_blocks = self.mainnet_blocks.lock().await;
                mainnet_blocks.entry(Height(block.height)).or_insert_with(Vec::new).push(block);
            }
            EventType::BlockConfirmation => {
                let block_confirmation: BlockConfirmationPayload = sonic_rs::from_str(&event.payload).unwrap();
                if block_confirmation.confirmations == 10 {
                    let mut mainnet_blocks = self.mainnet_blocks.lock().await;

                    // Look up the blocks at the confirmed height
                    if let Some(blocks) = mainnet_blocks.remove(&Height(block_confirmation.height)) {
                        for block in blocks {
                            for account in block.accounts().iter().filter(|a| !a.is_empty()) {
                                if block.state_hash == block_confirmation.state_hash {
                                    // Check if the account is already in the database
                                    let check_query = "SELECT EXISTS (SELECT 1 FROM discovered_accounts WHERE account = $1)";

                                    let account_check = self
                                        .client
                                        .query_one(check_query, &[&account])
                                        .await
                                        .map(|row| row.get::<_, bool>(0))
                                        .unwrap_or(false);

                                    if !account_check {
                                        // Publish a NewAccount event
                                        let new_account_event = Event {
                                            event_type: EventType::NewAccount,
                                            payload: sonic_rs::to_string(&NewAccountPayload {
                                                height: block.height,
                                                state_hash: block.state_hash.clone(),
                                                timestamp: block.timestamp,
                                                account: account.clone(),
                                            })
                                            .unwrap(),
                                        };
                                        self.publish(new_account_event);

                                        // Insert the account into the database
                                        let insert_query = "INSERT INTO discovered_accounts (account) VALUES ($1)";
                                        if let Err(e) = self.client.execute(insert_query, &[&account]).await {
                                            eprintln!("Failed to insert new account into database: {:?}", e);
                                        } else {
                                            self.database_inserts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    async fn report(&self) {
        let mainnet_blocks = self.mainnet_blocks.lock().await;
        self.print_report("Mainnet Blocks HashMap", mainnet_blocks.len());
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
        mainnet_block_models::CommandSummary,
        payloads::{BlockConfirmationPayload, MainnetBlockPayload, NewAccountPayload},
    };
    use std::sync::Arc;
    use tokio::time::timeout;

    async fn setup_actor() -> (Arc<NewAccountActor>, tokio::sync::broadcast::Receiver<Event>) {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = Arc::new(NewAccountActor::new(Arc::clone(&shared_publisher), false).await);
        let receiver = shared_publisher.subscribe();
        (actor, receiver)
    }

    #[tokio::test]
    async fn test_preexisting_account_inserted() {
        let (actor, _) = setup_actor().await;

        let account = "B62qtestaccount1".to_string();
        let event = Event {
            event_type: EventType::PreExistingAccount,
            payload: account.to_string(),
        };

        actor.handle_event(event).await;

        // Verify the account is inserted in the database
        let check_query = "SELECT EXISTS (SELECT 1 FROM discovered_accounts WHERE account = $1)";
        let account_exists: bool = actor.client.query_one(check_query, &[&account]).await.unwrap().get(0);

        assert!(account_exists, "Pre-existing account should be inserted into the database");
    }

    #[tokio::test]
    async fn test_mainnet_block_handling() {
        let (actor, _) = setup_actor().await;

        let block = MainnetBlockPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            user_commands: vec![CommandSummary {
                sender: "B62qaccount1".to_string(),
                receiver: "B62qaccount2".to_string(),
                ..Default::default()
            }],
            timestamp: 1234567890,
            ..Default::default()
        };

        let event = Event {
            event_type: EventType::MainnetBlock,
            payload: sonic_rs::to_string(&block).unwrap(),
        };

        actor.handle_event(event).await;

        // Verify the block is stored in the mainnet_blocks map
        let mainnet_blocks = actor.mainnet_blocks.lock().await;
        let stored_blocks = mainnet_blocks.get(&Height(block.height));
        assert!(stored_blocks.is_some(), "Mainnet block should be stored in memory");
        assert_eq!(stored_blocks.unwrap().len(), 1, "Mainnet block should contain one entry");
    }

    #[tokio::test]
    async fn test_block_confirmation_with_new_accounts() {
        let (actor, mut receiver) = setup_actor().await;

        let block = MainnetBlockPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            user_commands: vec![CommandSummary {
                sender: "B62qnewaccount".to_string(),
                receiver: "B62qnewaccount".to_string(),
                fee_payer: "B62qnewaccount".to_string(),
                ..Default::default()
            }],
            timestamp: 1234567890,
            ..Default::default()
        };

        // Add the mainnet block
        let block_event = Event {
            event_type: EventType::MainnetBlock,
            payload: sonic_rs::to_string(&block).unwrap(),
        };
        actor.handle_event(block_event).await;

        // Confirm the block
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

        // Verify a NewAccount event is published
        if let Ok(event) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            let received_event = event.unwrap();
            assert_eq!(received_event.event_type, EventType::NewAccount);

            let new_account_payload: NewAccountPayload = sonic_rs::from_str(&received_event.payload).unwrap();
            assert_eq!(new_account_payload.height, block.height);
            assert_eq!(new_account_payload.account, "B62qnewaccount".to_string());
        } else {
            panic!("Expected NewAccount event not received");
        }
    }

    #[tokio::test]
    async fn test_block_confirmation_with_existing_account() {
        let (actor, mut receiver) = setup_actor().await;

        let account = "B62qexistingaccount".to_string();

        // Add the preexisting account to the database
        let preexisting_event = Event {
            event_type: EventType::PreExistingAccount,
            payload: account.to_string(),
        };
        actor.handle_event(preexisting_event).await;

        let block = MainnetBlockPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            user_commands: vec![CommandSummary {
                sender: account.clone(),
                ..Default::default()
            }],
            timestamp: 1234567890,
            ..Default::default()
        };

        // Add the mainnet block
        let block_event = Event {
            event_type: EventType::MainnetBlock,
            payload: sonic_rs::to_string(&block).unwrap(),
        };
        actor.handle_event(block_event).await;

        // Confirm the block
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

        // Filter the events to ensure no NewAccount event is published for the sender
        let mut received_events = vec![];
        while let Ok(event) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            received_events.push(event.unwrap());
        }

        // Check for NewAccount events and ensure none is published for the sender
        let new_account_events: Vec<_> = received_events
            .iter()
            .filter(|event| {
                let event_payload: NewAccountPayload = sonic_rs::from_str(&event.payload).unwrap();
                event_payload.account == account
            })
            .collect();

        assert!(new_account_events.is_empty(), "No NewAccount event should be published for existing accounts");
    }
}
