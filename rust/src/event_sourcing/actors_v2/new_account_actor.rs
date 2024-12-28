use crate::{
    blockchain_tree::Height,
    constants::POSTGRES_CONNECTION_STRING,
    event_sourcing::{
        actor_dag::{ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
        events::{Event, EventType},
        payloads::{BlockConfirmationPayload, MainnetBlockPayload, NewAccountPayload},
    },
};
use log::error;
use std::collections::HashMap;
use tokio_postgres::{Client, NoTls};

/// Keys in the ActorStore
const CLIENT_KEY: &str = "new_account_client";
const BLOCKS_KEY: &str = "mainnet_blocks";

pub struct NewAccountActor;

impl NewAccountActor {
    /// Handle `EventType::PreExistingAccount`.
    /// Insert the account into a `discovered_accounts` table.
    async fn on_preexisting_account(account: String, store: &mut ActorStore) {
        let client = store.get::<Client>(CLIENT_KEY).expect("Postgres client missing from store");

        let insert_query = r#"
            INSERT INTO discovered_accounts (account, height) VALUES ($1, $2)
            ON CONFLICT DO NOTHING
        "#;

        if let Err(e) = client.execute(insert_query, &[&account, &0_i64]).await {
            error!("Failed to insert preexisting account {account}: {e}");
        }
    }

    /// Handle `EventType::MainnetBlock`.
    /// Add the block to `mainnet_blocks[block.height]`.
    async fn on_mainnet_block(block: MainnetBlockPayload, store: &mut ActorStore) {
        let mut blocks_map = store
            .remove::<HashMap<Height, Vec<MainnetBlockPayload>>>(BLOCKS_KEY)
            .expect("Mainnet blocks map missing from store");

        blocks_map.entry(Height(block.height)).or_default().push(block);

        store.insert::<HashMap<Height, Vec<MainnetBlockPayload>>>(BLOCKS_KEY, blocks_map);
    }

    /// Handle `EventType::BlockConfirmation`.
    /// If `confirmations == 10`, then for each block at that height,
    /// insert new discovered accounts if not already known, and produce a `NewAccount` event.
    async fn on_block_confirmation(conf: BlockConfirmationPayload, store: &mut ActorStore) -> Option<Vec<Event>> {
        if conf.confirmations != 10 {
            return None;
        }

        let client = store.remove::<Client>(CLIENT_KEY).expect("Postgres client missing from store");

        let mut blocks_map = store
            .remove::<HashMap<Height, Vec<MainnetBlockPayload>>>(BLOCKS_KEY)
            .expect("Mainnet blocks map missing from store");

        let mut out_events = Vec::new();

        // Look up blocks at the confirmed height:
        if let Some(blocks) = blocks_map.remove(&Height(conf.height)) {
            for block in blocks {
                // Only proceed if this block's state_hash matches the BlockConfirmation’s state_hash
                if block.state_hash != conf.state_hash {
                    continue;
                }

                // For each discovered account in the block
                for acct in block.valid_accounts().iter().filter(|a| !a.is_empty()) {
                    // Check if it exists in discovered_accounts
                    let check_query = "SELECT EXISTS (SELECT 1 FROM discovered_accounts WHERE account = $1)";
                    let account_exists = client.query_one(check_query, &[&acct]).await.map(|row| row.get::<_, bool>(0)).unwrap_or(false);

                    if !account_exists {
                        // Insert the account into discovered_accounts
                        let insert_query = "INSERT INTO discovered_accounts (account, height) VALUES ($1, $2)";
                        if let Err(e) = client.execute(insert_query, &[&acct, &(conf.height as i64)]).await {
                            error!("Failed to insert new account {acct} at height={} into database: {e}", conf.height);
                            continue;
                        }

                        // Also produce a `NewAccount` event so that other actors can see it
                        out_events.push(Event {
                            event_type: EventType::NewAccount,
                            payload: sonic_rs::to_string(&NewAccountPayload {
                                height: block.height,
                                state_hash: block.state_hash.clone(),
                                timestamp: block.timestamp,
                                account: acct.clone(),
                            })
                            .unwrap(),
                        });
                    }
                }
            }
        }

        store.insert::<Client>(CLIENT_KEY, client);
        store.insert::<HashMap<Height, Vec<MainnetBlockPayload>>>(BLOCKS_KEY, blocks_map);

        if out_events.is_empty() {
            None
        } else {
            Some(out_events)
        }
    }
}

#[async_trait::async_trait]
impl ActorFactory for NewAccountActor {
    async fn create_actor() -> ActorNode {
        // 1) Connect to Postgres
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Unable to connect to database in NewAccountActor");

        // Spawn the connection task
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("Postgres connection error: {}", e);
            }
        });

        // 2) Create discovered_accounts table if needed
        let create_table_sql = r#"
            CREATE TABLE IF NOT EXISTS discovered_accounts (
                account TEXT PRIMARY KEY NOT NULL,
                height BIGINT NOT NULL
            );
        "#;
        if let Err(e) = client.execute(create_table_sql, &[]).await {
            error!("Unable to create discovered_accounts table: {e}");
        }

        // 3) Build store with the client + the mainnet_blocks map
        let mut store = ActorStore::new();
        store.insert(CLIENT_KEY, client);
        store.insert(BLOCKS_KEY, HashMap::<Height, Vec<MainnetBlockPayload>>::new());

        // 4) Build the node with a single event processor closure:
        ActorNodeBuilder::new()
            .with_state(store)
            .with_processor(|event, store, _requeue| {
                Box::pin(async move {
                    let mut locked_store = store.lock().await;

                    match event.event_type {
                        EventType::PreExistingAccount => {
                            let acct = event.payload; // just a string
                            NewAccountActor::on_preexisting_account(acct, &mut locked_store).await;
                            None
                        }

                        EventType::MainnetBlock => {
                            let block: MainnetBlockPayload = sonic_rs::from_str(&event.payload).expect("Failed to parse MainnetBlockPayload");
                            NewAccountActor::on_mainnet_block(block, &mut locked_store).await;
                            None
                        }

                        EventType::BlockConfirmation => {
                            let bc: BlockConfirmationPayload = sonic_rs::from_str(&event.payload).expect("Failed to parse BlockConfirmationPayload");
                            // If some events are produced (i.e. new accounts discovered), return them
                            NewAccountActor::on_block_confirmation(bc, &mut locked_store).await
                        }

                        _ => None,
                    }
                })
            })
            .build()
    }
}

#[cfg(test)]
mod new_account_actor_tests_v2 {
    use super::NewAccountActor;
    use crate::{
        blockchain_tree::Height,
        constants::POSTGRES_CONNECTION_STRING,
        event_sourcing::{
            actor_dag::*,
            events::{Event, EventType},
            models::{CommandStatus, CommandSummary},
            payloads::{BlockConfirmationPayload, MainnetBlockPayload, NewAccountPayload},
        },
    };
    use log::error;
    use std::{collections::HashMap, sync::Arc};
    use tokio::{
        sync::Mutex,
        time::{sleep, Duration},
    };
    use tokio_postgres::{Client, NoTls};

    async fn setup() {
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Unable to connect to database");
        // Spawn the connection task
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("Postgres connection error: {}", e);
            }
        });

        if let Err(e) = client.execute("DROP TABLE IF EXISTS discovered_accounts;", &[]).await {
            error!("Unable to drop user_commands table {:?}", e);
        }
    }

    // HELPER: Create a node that captures `NewAccount` events
    fn create_new_account_sink_node() -> ActorNode {
        ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|event, store, _requeue| {
                Box::pin(async move {
                    if event.event_type == EventType::NewAccount {
                        let mut locked_store = store.lock().await;
                        let mut captured: Vec<String> = locked_store.get("captured_new_accounts").cloned().unwrap_or_default();
                        captured.push(event.payload.clone());
                        locked_store.insert("captured_new_accounts", captured);
                    }
                    None
                })
            })
            .build()
    }

    /// Helper to read captured `NewAccount` event payloads from the sink node
    async fn read_captured_new_accounts(dag: &ActorDAG, sink_node_id: &str) -> Vec<String> {
        let node_arc = dag.read_node(sink_node_id.to_string()).expect("Sink node not found");
        let node_guard = node_arc.lock().await;
        let store = node_guard.get_state();
        let locked_store = store.lock().await;
        locked_store.get::<Vec<String>>("captured_new_accounts").cloned().unwrap_or_default()
    }

    #[tokio::test]
    async fn test_preexisting_account_inserted() {
        setup().await;
        // 1. Build the DAG
        let mut dag = ActorDAG::new();

        // 2. Create the NewAccountActor (root)
        let new_account_actor = NewAccountActor::create_actor().await;
        let new_account_id = new_account_actor.id();
        let sender = dag.set_root(new_account_actor);

        // 3. Create a sink node

        let sink_node = create_new_account_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&new_account_id, sink_node_id);

        // 4. Wrap + spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 5. Send PreExistingAccount event
        let account = "B62qtestaccount1".to_string();
        sender
            .send(Event {
                event_type: EventType::PreExistingAccount,
                payload: account.clone(),
            })
            .await
            .expect("Failed to send event");

        // Wait a bit
        sleep(Duration::from_millis(200)).await;

        // 6. Verify the discovered_accounts table directly
        let dag_locked = dag.lock().await;
        let node_arc = dag_locked.read_node(new_account_id).expect("Node not found");
        let node_guard = node_arc.lock().await;
        let store = node_guard.get_state();
        let mut locked_store = store.lock().await;

        // Grab the Postgres client
        let client = locked_store.remove::<Client>("new_account_client").expect("Missing client in store");
        let check_query = "SELECT EXISTS (SELECT 1 FROM discovered_accounts WHERE account = $1)";
        let account_exists: bool = client.query_one(check_query, &[&account]).await.unwrap().get(0);
        assert!(account_exists, "Pre-existing account should be inserted into the DB");

        // Reinsert the client so it remains in the store
        locked_store.insert("new_account_client", client);
    }

    #[tokio::test]
    async fn test_mainnet_block_handling() {
        setup().await;
        // 1. Build the DAG
        let mut dag = ActorDAG::new();

        // 2. Create the NewAccountActor (root)
        let new_account_actor = NewAccountActor::create_actor().await;
        let new_account_id = new_account_actor.id();
        let sender = dag.set_root(new_account_actor);

        // 3. We'll skip the sink node, since we only want to check in-memory block storage (But if you like, you could still add it for coverage.)

        // 4. Spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 5. Send a MainnetBlock event
        let test_block = MainnetBlockPayload {
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

        sender
            .send(Event {
                event_type: EventType::MainnetBlock,
                payload: sonic_rs::to_string(&test_block).unwrap(),
            })
            .await
            .expect("Failed to send mainnet block event");

        // Wait
        sleep(Duration::from_millis(200)).await;

        // 6. Read store => confirm blocks map
        let dag_locked = dag.lock().await;
        let node_arc = dag_locked.read_node(new_account_id).expect("Node not found");
        let node_guard = node_arc.lock().await;
        let store = node_guard.get_state();
        let locked_store = store.lock().await;

        // The blocks map is stored under "mainnet_blocks"
        let blocks_map = locked_store
            .get::<HashMap<Height, Vec<MainnetBlockPayload>>>("mainnet_blocks")
            .expect("blocks map missing from store");

        let stored = blocks_map.get(&Height(1));
        assert!(stored.is_some(), "Mainnet block should be stored in memory");
        assert_eq!(stored.unwrap().len(), 1, "We expect exactly 1 block at height=1");
    }

    #[tokio::test]
    async fn test_block_confirmation_with_new_accounts() {
        setup().await;
        // 1. Build the DAG
        let mut dag = ActorDAG::new();

        // 2. Create the NewAccountActor + sink node
        let new_account_actor = NewAccountActor::create_actor().await;
        let new_account_id = new_account_actor.id();
        let sender = dag.set_root(new_account_actor);

        let sink_node = create_new_account_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&new_account_id, sink_node_id);

        // 3. Spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 4. Send a MainnetBlock with an "Applied" command
        let block = MainnetBlockPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            user_commands: vec![CommandSummary {
                sender: "B62qnewaccount".to_string(),
                receiver: "B62qnewaccount".to_string(),
                fee_payer: "B62qnewaccount".to_string(),
                status: CommandStatus::Applied,
                ..Default::default()
            }],
            timestamp: 1234567890,
            ..Default::default()
        };
        sender
            .send(Event {
                event_type: EventType::MainnetBlock,
                payload: sonic_rs::to_string(&block).unwrap(),
            })
            .await
            .unwrap();

        // 5. Now confirm => confirmations=10
        let conf = BlockConfirmationPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            confirmations: 10,
        };
        sender
            .send(Event {
                event_type: EventType::BlockConfirmation,
                payload: sonic_rs::to_string(&conf).unwrap(),
            })
            .await
            .unwrap();

        // Wait
        sleep(Duration::from_millis(200)).await;

        // 6. Read from the sink node
        let dag_locked = dag.lock().await;
        let captured = read_captured_new_accounts(&dag_locked, sink_node_id).await;
        assert_eq!(captured.len(), 1, "Expected exactly one `NewAccount` event captured");
        let first_payload = &captured[0];
        let new_account_payload: NewAccountPayload = sonic_rs::from_str(first_payload).unwrap();
        assert_eq!(new_account_payload.account, "B62qnewaccount");
        assert_eq!(new_account_payload.height, 1);
    }

    #[tokio::test]
    async fn test_block_confirmation_with_new_accounts_failed_command() {
        setup().await;
        // 1. DAG
        let mut dag = ActorDAG::new();

        // 2. Root node + sink
        let new_account_actor = NewAccountActor::create_actor().await;
        let new_account_id = new_account_actor.id();
        let sender = dag.set_root(new_account_actor);

        let sink_node = create_new_account_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&new_account_id, sink_node_id);

        // 3. Spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 4. Add a block with "Failed" user command
        let block = MainnetBlockPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            user_commands: vec![CommandSummary {
                sender: "B62qnewaccount".to_string(),
                receiver: "B62qnewaccount".to_string(),
                fee_payer: "B62qnewaccount".to_string(),
                status: CommandStatus::Failed,
                ..Default::default()
            }],
            timestamp: 1234567890,
            ..Default::default()
        };
        sender
            .send(Event {
                event_type: EventType::MainnetBlock,
                payload: sonic_rs::to_string(&block).unwrap(),
            })
            .await
            .unwrap();

        // 5. Confirm => 10
        let conf = BlockConfirmationPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            confirmations: 10,
        };
        sender
            .send(Event {
                event_type: EventType::BlockConfirmation,
                payload: sonic_rs::to_string(&conf).unwrap(),
            })
            .await
            .unwrap();

        // Wait
        sleep(Duration::from_millis(200)).await;

        // 6. Check the sink => it should have captured ZERO NewAccount events
        let dag_locked = dag.lock().await;
        let captured = read_captured_new_accounts(&dag_locked, sink_node_id).await;
        assert!(captured.is_empty(), "Expected no `NewAccount` event for failed command");
    }

    #[tokio::test]
    async fn test_block_confirmation_with_existing_account() {
        setup().await;
        // 1. DAG
        let mut dag = ActorDAG::new();

        // 2. Root + sink
        let new_account_actor = NewAccountActor::create_actor().await;
        let new_account_id = new_account_actor.id();
        let sender = dag.set_root(new_account_actor);

        let sink_node = create_new_account_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&new_account_id, sink_node_id);

        // 3. Spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 4. Insert a preexisting account
        sender
            .send(Event {
                event_type: EventType::PreExistingAccount,
                payload: "B62qexistingaccount".to_string(),
            })
            .await
            .unwrap();
        sleep(Duration::from_millis(100)).await;

        // 5. Add a block referencing that existing account
        let block = MainnetBlockPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            user_commands: vec![CommandSummary {
                sender: "B62qexistingaccount".to_string(),
                ..Default::default()
            }],
            timestamp: 1234567890,
            ..Default::default()
        };
        sender
            .send(Event {
                event_type: EventType::MainnetBlock,
                payload: sonic_rs::to_string(&block).unwrap(),
            })
            .await
            .unwrap();

        // 6. Confirm => 10
        let conf = BlockConfirmationPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            confirmations: 10,
        };
        sender
            .send(Event {
                event_type: EventType::BlockConfirmation,
                payload: sonic_rs::to_string(&conf).unwrap(),
            })
            .await
            .unwrap();

        // Wait
        sleep(Duration::from_millis(200)).await;

        // 7. Check the sink => no new accounts
        let dag_locked = dag.lock().await;
        let captured = read_captured_new_accounts(&dag_locked, sink_node_id).await;
        assert!(captured.is_empty(), "No NewAccount event should be published for existing accounts");
    }
}
