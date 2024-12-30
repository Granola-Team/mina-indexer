use crate::{
    blockchain_tree::Height,
    constants::POSTGRES_CONNECTION_STRING,
    event_sourcing::{
        actor_dag::{ActorNode, ActorNodeBuilder, ActorStore},
        events::EventType,
        managed_store::ManagedStore,
        payloads::{BlockConfirmationPayload, MainnetBlockPayload, NewAccountPayload},
    },
};
use log::error;
use std::collections::HashMap;
use tokio_postgres::NoTls;

/// The key we use to store the ManagedStore in the ActorStore
const ACCOUNTS_STORE_KEY: &str = "discovered_accounts_store";
/// The key we use to store in-memory `HashMap<Height, Vec<MainnetBlockPayload>>`
const BLOCKS_KEY: &str = "mainnet_blocks";

/// Our actor that logs new accounts once a block is confirmed at 10 confirmations.
pub struct NewAccountActor;

impl NewAccountActor {
    /// Handle a `PreExistingAccount` event by upserting that account with `height=0`.
    async fn on_preexisting_account(account: String, store: &mut ActorStore) {
        let managed_store = store.remove::<ManagedStore>(ACCOUNTS_STORE_KEY).expect("ManagedStore missing from store");

        // We'll store them as `key=<account>`, and set "height" = 0 if it's new.
        // If the row already exists, we do nothing (the store logic will update only the columns we supply).
        // We'll assume columns: key TEXT PRIMARY KEY, height BIGINT
        let pairs = &[("height", &0_i64 as &(dyn tokio_postgres::types::ToSql + Sync))];

        if let Err(e) = managed_store.upsert(&account, pairs).await {
            error!("Failed to upsert preexisting account={}: {}", account, e);
        }

        store.insert::<ManagedStore>(ACCOUNTS_STORE_KEY, managed_store);
    }

    /// Handle `MainnetBlock` => store it in the in-memory `HashMap<Height, Vec<MainnetBlockPayload>>`.
    async fn on_mainnet_block(block: MainnetBlockPayload, store: &mut ActorStore) {
        // Retrieve the blocks map
        let mut blocks_map = store
            .remove::<HashMap<Height, Vec<MainnetBlockPayload>>>(BLOCKS_KEY)
            .expect("Missing blocks map from store");

        blocks_map.entry(Height(block.height)).or_default().push(block);

        // Put it back
        store.insert(BLOCKS_KEY, blocks_map);
    }

    /// Handle `BlockConfirmation`.
    /// If confirmations == 10, we discover new accounts from that block (i.e. each block’s `valid_accounts()`).
    /// We upsert them to the ManagedStore with `height = conf.height`, if they don’t already exist.
    async fn on_block_confirmation(conf: BlockConfirmationPayload, store: &mut ActorStore) -> Option<Vec<crate::event_sourcing::events::Event>> {
        if conf.confirmations != 10 {
            return None;
        }

        // Grab the store
        let managed_store = store.remove::<ManagedStore>(ACCOUNTS_STORE_KEY).expect("Missing ManagedStore in store");

        // Grab the blocks map
        let mut blocks_map = store
            .remove::<HashMap<Height, Vec<MainnetBlockPayload>>>(BLOCKS_KEY)
            .expect("Missing blocks map in store");

        let mut out_events = Vec::new();

        if let Some(blocks) = blocks_map.remove(&Height(conf.height)) {
            for block in blocks {
                // Must match the state_hash
                if block.state_hash != conf.state_hash {
                    continue;
                }

                // For each discovered account
                for acct in block.valid_accounts().iter().filter(|a| !a.is_empty()) {
                    // Upsert the account if it doesn’t exist, with `height=conf.height`
                    // If the row already exists, we do not override it if the store logic is set up that way
                    // (But you might want to store the *minimum* discovered height, or the *maximum*, or something else.)
                    let pairs = &[("height", &(conf.height as i64) as &(dyn tokio_postgres::types::ToSql + Sync))];

                    // Upsert
                    let res = managed_store.upsert(acct, pairs).await;
                    if let Err(e) = res {
                        error!("Failed to upsert new account={} at height={}: {}", acct, conf.height, e);
                        continue;
                    }

                    // We also produce a `NewAccount` event so that other actors can see it
                    out_events.push(crate::event_sourcing::events::Event {
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

        // Put things back
        store.insert::<ManagedStore>(ACCOUNTS_STORE_KEY, managed_store);
        store.insert(BLOCKS_KEY, blocks_map);

        if out_events.is_empty() {
            None
        } else {
            Some(out_events)
        }
    }

    pub async fn create_actor(preserve_data: bool) -> ActorNode {
        // 1) Connect to Postgres
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Unable to connect to DB in NewAccountActor");
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("Postgres connection error in NewAccountActor: {}", e);
            }
        });

        // 2) Build (or re-build) our ManagedStore for discovered accounts.
        // Suppose we define columns: key TEXT PRIMARY KEY, height BIGINT
        // Non-preserving => we drop the table each time, unless you want to set preserve_data().
        let store_builder = ManagedStore::builder(client).name(ACCOUNTS_STORE_KEY).add_numeric_column("height"); // default=0
        let managed_store = if preserve_data {
            store_builder
                .preserve_data()
                .build()
                .await
                .expect("Failed to build {ACCOUNTS_STORE_KEY} ManagedStore")
        } else {
            store_builder.build().await.expect("Failed to build {ACCOUNTS_STORE_KEY} ManagedStore")
        };

        // 3) Create an empty HashMap<Height, Vec<MainnetBlockPayload>>
        let blocks_map: HashMap<Height, Vec<MainnetBlockPayload>> = HashMap::new();

        // 4) Put them in the actor store
        let mut store = ActorStore::new();
        store.insert::<ManagedStore>(ACCOUNTS_STORE_KEY, managed_store);
        store.insert(BLOCKS_KEY, blocks_map);

        // 5) Build and return the ActorNode
        ActorNodeBuilder::new()
            .with_state(store)
            .with_processor(|event, actor_store, _requeue| {
                Box::pin(async move {
                    let mut locked_store = actor_store.lock().await;

                    match event.event_type {
                        EventType::PreExistingAccount => {
                            let account_str = event.payload; // raw String
                            NewAccountActor::on_preexisting_account(account_str, &mut locked_store).await;
                            None
                        }
                        EventType::MainnetBlock => {
                            let block: MainnetBlockPayload = sonic_rs::from_str(&event.payload).expect("Failed to parse MainnetBlockPayload");
                            NewAccountActor::on_mainnet_block(block, &mut locked_store).await;
                            None
                        }
                        EventType::BlockConfirmation => {
                            let conf: BlockConfirmationPayload = sonic_rs::from_str(&event.payload).expect("Failed to parse BlockConfirmationPayload");
                            // Possibly produce `NewAccount` events
                            NewAccountActor::on_block_confirmation(conf, &mut locked_store).await
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
    use super::{NewAccountActor, ACCOUNTS_STORE_KEY};
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
    use std::{collections::HashMap, sync::Arc};
    use tokio::{
        sync::Mutex,
        time::{sleep, Duration},
    };
    use tokio_postgres::NoTls;

    /// Connect to Postgres with the standard `POSTGRES_CONNECTION_STRING`.
    /// Spawns the connection handling on a background task.
    async fn connect_to_db() -> tokio_postgres::Client {
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Failed to connect to PostgreSQL");

        // Spawn the connection so errors are logged if they occur.
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        client
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
        // 1. Build the DAG
        let mut dag = ActorDAG::new();

        // 2. Create the NewAccountActor (root)
        let new_account_actor = NewAccountActor::create_actor(false).await;
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

        // Grab the Postgres client
        let client = connect_to_db().await;
        let check_query = &format!("SELECT EXISTS (SELECT 1 FROM {} WHERE key = $1)", ACCOUNTS_STORE_KEY);
        let account_exists: bool = client.query_one(check_query, &[&account]).await.unwrap().get(0);
        assert!(account_exists, "Pre-existing account should be inserted into the DB");
    }

    #[tokio::test]
    async fn test_mainnet_block_handling() {
        // 1. Build the DAG
        let mut dag = ActorDAG::new();

        // 2. Create the NewAccountActor (root)
        let new_account_actor = NewAccountActor::create_actor(false).await;
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
        // 1. Build the DAG
        let mut dag = ActorDAG::new();

        // 2. Create the NewAccountActor + sink node
        let new_account_actor = NewAccountActor::create_actor(false).await;
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
        // 1. DAG
        let mut dag = ActorDAG::new();

        // 2. Root node + sink
        let new_account_actor = NewAccountActor::create_actor(false).await;
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
        // 1. DAG
        let mut dag = ActorDAG::new();

        // 2. Root + sink
        let new_account_actor = NewAccountActor::create_actor(false).await;
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
