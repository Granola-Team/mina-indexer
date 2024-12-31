use crate::{
    constants::POSTGRES_CONNECTION_STRING,
    event_sourcing::{
        actor_dag::{ActorNode, ActorNodeBuilder, ActorStore},
        canonical_items_manager::CanonicalItem,
        events::{Event, EventType},
        managed_store::ManagedStore,
        payloads::{CanonicalMainnetBlockPayload, NewAccountPayload},
    },
};
use log::{debug, error};
use tokio_postgres::NoTls;

/// The key we use to store the ManagedStore in the ActorStore
const ACCOUNTS_STORE_KEY: &str = "discovered_accounts_store";

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

    async fn on_mainnet_block(block: CanonicalMainnetBlockPayload, store: &mut ActorStore) -> Option<Vec<crate::event_sourcing::events::Event>> {
        // Grab the store
        let managed_store = store.remove::<ManagedStore>(ACCOUNTS_STORE_KEY).expect("Missing ManagedStore in store");

        let mut out_events = Vec::new();

        // For each discovered account
        for acct in block.valid_accounts().iter().filter(|a| !a.is_empty()) {
            let maybe_height = managed_store.get::<i64>(acct, "height").await;
            let maybe_state = managed_store.get::<String>(acct, "state_hash").await;

            match (maybe_height, maybe_state, block.canonical) {
                (Ok(None), Ok(None), true) => {
                    let pairs = &[
                        ("height", &(block.block.height as i64) as &(dyn tokio_postgres::types::ToSql + Sync)),
                        ("state_hash", &block.block.state_hash as &(dyn tokio_postgres::types::ToSql + Sync)),
                    ];
                    let res = managed_store.upsert(acct, pairs).await;
                    if let Err(e) = res {
                        error!("Failed to upsert new account={} at height={}: {}", acct, block.block.height, e);
                        continue;
                    }

                    // We also produce a `NewAccount` event so that other actors can see it
                    out_events.push(Event {
                        event_type: EventType::NewAccount,
                        payload: sonic_rs::to_string(&NewAccountPayload {
                            apply: true,
                            height: block.block.height,
                            state_hash: block.block.state_hash.clone(),
                            timestamp: block.block.timestamp,
                            account: acct.clone(),
                        })
                        .unwrap(),
                    });
                }
                (Ok(Some(_)), Ok(Some(_)), true) => {
                    debug!("Account already discovered: {acct}");
                }
                (Ok(Some(height)), Ok(Some(state_hash)), false) if block.get_height() == height as u64 && block.get_state_hash() == state_hash => {
                    if let Err(e) = managed_store.remove_key(acct).await {
                        error!("Unable to remove from store: {e}");
                    }
                    // We also produce a `NewAccount` event so that other actors can see it
                    out_events.push(Event {
                        event_type: EventType::NewAccount,
                        payload: sonic_rs::to_string(&NewAccountPayload {
                            apply: false,
                            height: block.block.height,
                            state_hash: block.block.state_hash.clone(),
                            timestamp: block.block.timestamp,
                            account: acct.clone(),
                        })
                        .unwrap(),
                    });
                }
                _ => {}
            }
        }

        // Put things back
        store.insert::<ManagedStore>(ACCOUNTS_STORE_KEY, managed_store);

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
        let store_builder = ManagedStore::builder(client)
            .name(ACCOUNTS_STORE_KEY)
            .add_text_column("state_hash")
            .add_numeric_column("height"); // default=0
        let managed_store = if preserve_data {
            store_builder
                .preserve_data()
                .build()
                .await
                .expect("Failed to build {ACCOUNTS_STORE_KEY} ManagedStore")
        } else {
            store_builder.build().await.expect("Failed to build {ACCOUNTS_STORE_KEY} ManagedStore")
        };

        // 4) Put them in the actor store
        let mut store = ActorStore::new();
        store.insert::<ManagedStore>(ACCOUNTS_STORE_KEY, managed_store);

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
                        EventType::CanonicalMainnetBlock => {
                            let block: CanonicalMainnetBlockPayload = sonic_rs::from_str(&event.payload).expect("Failed to parse MainnetBlockPayload");
                            NewAccountActor::on_mainnet_block(block, &mut locked_store).await
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
        constants::POSTGRES_CONNECTION_STRING,
        event_sourcing::{
            actor_dag::*,
            events::{Event, EventType},
            models::{CommandStatus, CommandSummary},
            payloads::{CanonicalMainnetBlockPayload, MainnetBlockPayload, NewAccountPayload},
        },
    };
    use std::sync::Arc;
    use tokio::{
        sync::Mutex,
        time::{sleep, Duration},
    };
    use tokio_postgres::NoTls;

    // ----------------------------------------------------------------
    // HELPER: Connect to Postgres
    // ----------------------------------------------------------------
    async fn connect_to_db() -> tokio_postgres::Client {
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Failed to connect to PostgreSQL");

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        client
    }

    // ----------------------------------------------------------------
    // HELPER: Create sink node to capture `NewAccount` events
    // ----------------------------------------------------------------
    fn create_new_account_sink_node() -> ActorNode {
        ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|event, state, _requeue| {
                Box::pin(async move {
                    if event.event_type == EventType::NewAccount {
                        let mut locked_state = state.lock().await;
                        let mut captured: Vec<String> = locked_state.get("captured_new_accounts").cloned().unwrap_or_default();
                        captured.push(event.payload.clone());
                        locked_state.insert("captured_new_accounts", captured);
                    }
                    None
                })
            })
            .build()
    }

    async fn read_captured_new_accounts(dag: &ActorDAG, sink_node_id: &str) -> Vec<String> {
        let node_arc = dag.read_node(sink_node_id.to_string()).expect("Sink node not found");
        let node_guard = node_arc.lock().await;
        let store = node_guard.get_state();
        let locked_store = store.lock().await;
        locked_store.get::<Vec<String>>("captured_new_accounts").cloned().unwrap_or_default()
    }

    #[tokio::test]
    async fn test_preexisting_account_inserted() {
        // Create the ActorDAG
        let mut dag = ActorDAG::new();

        // Root: create the actor
        let new_account_actor = NewAccountActor::create_actor(false).await;
        let actor_id = new_account_actor.id();
        let sender = dag.set_root(new_account_actor);

        // Sink node
        let sink_node = create_new_account_sink_node();
        let sink_node_id = sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, &sink_node_id);

        // Wrap + spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // Send a `PreExistingAccount` event
        let account = "B62qTestPreexisting".to_string();
        sender
            .send(Event {
                event_type: EventType::PreExistingAccount,
                payload: account.clone(),
            })
            .await
            .unwrap();

        // Wait
        sleep(Duration::from_millis(200)).await;

        // Now check Postgres to confirm the account is in the store
        let client = connect_to_db().await;
        let check_q = format!("SELECT EXISTS (SELECT 1 FROM {} WHERE key = $1)", ACCOUNTS_STORE_KEY);
        let exists: bool = client.query_one(&check_q, &[&account]).await.unwrap().get(0);
        assert!(exists, "PreExistingAccount should be inserted in the store");
    }

    #[tokio::test]
    async fn test_canonical_block_failed_command() {
        // Build DAG
        let mut dag = ActorDAG::new();
        let actor = NewAccountActor::create_actor(false).await;
        let actor_id = actor.id();
        let sender = dag.set_root(actor);

        let sink_node = create_new_account_sink_node();
        let sink_id = sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, &sink_id);

        // Spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move { dag.lock().await.spawn_all().await }
        });

        // Build a CanonicalMainnetBlockPayload with a user command that is "Failed"
        // We'll set canonical=true => but the command is failed => no new account
        let block_payload = CanonicalMainnetBlockPayload {
            block: crate::event_sourcing::payloads::MainnetBlockPayload {
                height: 10,
                state_hash: "hash_fail_cmd".into(),
                user_commands: vec![CommandSummary {
                    sender: "B62qFailSender".to_string(),
                    receiver: "B62qFailReceiver".to_string(),
                    fee_payer: "B62qFailFeePayer".to_string(),
                    status: CommandStatus::Failed,
                    ..Default::default()
                }],
                timestamp: 555555,
                ..Default::default()
            },
            canonical: true,
            was_canonical: false, // newly canonical
        };

        // Send this event
        sender
            .send(Event {
                event_type: EventType::CanonicalMainnetBlock,
                payload: sonic_rs::to_string(&block_payload).unwrap(),
            })
            .await
            .expect("Failed to send canonical block with failed command");

        // Wait
        sleep(Duration::from_millis(300)).await;

        // Check the sink => we expect NO NewAccount events
        let dag_locked = dag.lock().await;
        let captured_events = read_captured_new_accounts(&dag_locked, &sink_id).await;
        assert!(captured_events.is_empty(), "No new accounts should be discovered if the command is 'Failed'.");
    }

    #[tokio::test]
    async fn test_account_discovered_once_across_multiple_canonical_blocks() {
        // DAG
        let mut dag = ActorDAG::new();
        let actor = NewAccountActor::create_actor(false).await;
        let actor_id = actor.id();
        let sender = dag.set_root(actor);

        let sink_node = create_new_account_sink_node();
        let sink_id = sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, &sink_id);

        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move { dag.lock().await.spawn_all().await }
        });

        // The repeated account
        let repeated_acct = "B62qRepeatAcct".to_string();

        // We'll create 2 canonical blocks referencing the same account
        let block1 = CanonicalMainnetBlockPayload {
            block: crate::event_sourcing::payloads::MainnetBlockPayload {
                height: 20,
                state_hash: "hash_can1".into(),
                user_commands: vec![CommandSummary {
                    sender: repeated_acct.clone(),
                    receiver: repeated_acct.clone(),
                    fee_payer: repeated_acct.clone(),
                    status: CommandStatus::Applied,
                    ..Default::default()
                }],
                timestamp: 111111,
                ..Default::default()
            },
            canonical: true,
            was_canonical: false,
        };
        let block2 = CanonicalMainnetBlockPayload {
            block: crate::event_sourcing::payloads::MainnetBlockPayload {
                height: 21,
                state_hash: "hash_can2".into(),
                user_commands: vec![CommandSummary {
                    sender: repeated_acct.clone(),
                    receiver: repeated_acct.clone(),
                    fee_payer: repeated_acct.clone(),
                    status: CommandStatus::Applied,
                    ..Default::default()
                }],
                timestamp: 222222,
                ..Default::default()
            },
            canonical: true,
            was_canonical: false,
        };

        // Send block1 => canonical => new account discovered
        sender
            .send(Event {
                event_type: EventType::CanonicalMainnetBlock,
                payload: sonic_rs::to_string(&block1).unwrap(),
            })
            .await
            .unwrap();

        // Wait
        sleep(Duration::from_millis(200)).await;

        // Send block2 => also canonical => same account
        sender
            .send(Event {
                event_type: EventType::CanonicalMainnetBlock,
                payload: sonic_rs::to_string(&block2).unwrap(),
            })
            .await
            .unwrap();

        // Wait
        sleep(Duration::from_millis(300)).await;

        // Check how many "NewAccount" events we captured for repeated_acct
        let dag_locked = dag.lock().await;
        let all_payloads = read_captured_new_accounts(&dag_locked, &sink_id).await;

        let mut relevant = vec![];
        for p in &all_payloads {
            let nap: NewAccountPayload = sonic_rs::from_str(p).unwrap();
            if nap.account == repeated_acct {
                relevant.push(nap);
            }
        }

        // We expect EXACTLY 1 discover event, the second block should skip
        assert_eq!(relevant.len(), 1, "The same account in multiple canonical blocks => discovered only once");

        // Also confirm the final stored height is from the first block (20), not overwritten
        let client = connect_to_db().await;
        let query = "SELECT height FROM discovered_accounts_store WHERE key=$1";
        let row_opt = client.query_opt(query, &[&repeated_acct]).await.unwrap();
        assert!(row_opt.is_some(), "Should have an entry in discovered_accounts_store for repeated_acct");
        let row = row_opt.unwrap();
        let final_height: i64 = row.get("height");
        assert_eq!(final_height, 20, "Should not overwrite the original discovery height for the repeated account");
    }

    #[tokio::test]
    async fn test_non_canonical_referencing_non_existent_account() {
        // 1) Build the DAG + root actor
        let mut dag = ActorDAG::new();

        // Create the actor with `preserve_data = false` (adjust as needed).
        let actor_node = NewAccountActor::create_actor(false).await;
        let actor_id = actor_node.id();
        let sender = dag.set_root(actor_node);

        // 2) Create the sink node for capturing NewAccount events, link it
        let sink_node = create_new_account_sink_node();
        let sink_node_id = sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, &sink_node_id);

        // 3) Wrap + spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 4) Construct a block referencing an unknown account => canonical=false, was_canonical=false
        let block_payload = CanonicalMainnetBlockPayload {
            block: MainnetBlockPayload {
                height: 50,
                state_hash: "some_noncanon_hash".to_string(),
                user_commands: vec![CommandSummary {
                    sender: "B62qGhostAccount".to_string(),   // an account that doesn't exist
                    receiver: "B62qGhostAccount".to_string(), // same
                    status: CommandStatus::Applied,
                    ..Default::default()
                }],
                timestamp: 123456,
                ..Default::default()
            },
            canonical: false,
            was_canonical: false, // purely non-canonical, never was canonical
        };

        // 5) Send this event
        sender
            .send(Event {
                event_type: EventType::CanonicalMainnetBlock,
                payload: sonic_rs::to_string(&block_payload).unwrap(),
            })
            .await
            .expect("Failed to send non-canonical block referencing non-existent account");

        // Wait a bit
        tokio::time::sleep(Duration::from_millis(200)).await;

        // 6) Check sink => we expect no `NewAccount` events since there's no store entry to remove
        let dag_locked = dag.lock().await;
        let captured = read_captured_new_accounts(&dag_locked, &sink_node_id).await;
        assert!(
            captured.is_empty(),
            "Non-canonical referencing a non-existent account should produce no new account events"
        );

        // Also no error should have occurred, but that’s implied if the test completes.
    }

    #[tokio::test]
    async fn test_non_canonical_block_at_same_height_for_undiscovered_account() {
        // 1) Build the DAG, root actor, sink node, etc.
        let mut dag = ActorDAG::new();

        // Create the actor with `preserve_data = false` (or `true` if you prefer).
        let actor_node = NewAccountActor::create_actor(false).await;
        let actor_id = actor_node.id();
        let sender = dag.set_root(actor_node);

        let sink_node = create_new_account_sink_node();
        let sink_node_id = sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, &sink_node_id);

        // Wrap + spawn the DAG
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 2) Insert a preexisting account at height=0
        sender
            .send(Event {
                event_type: EventType::PreExistingAccount,
                payload: "B62qPhantomAccount".to_string(),
            })
            .await
            .unwrap();

        // 3) Create a *non-canonical* block at height=50 referencing that account but the code believes it was once canonical ("was_canonical=true"), so it
        //    tries to "revert" it. We expect no removal since we never had the account discovered at height=50 in the store.
        let test_block = MainnetBlockPayload {
            height: 50,
            state_hash: "phantom_noncanon_hash".to_string(),
            user_commands: vec![CommandSummary {
                sender: "B62qPhantomAccount".to_string(),
                receiver: "B62qPhantomAccount".to_string(),
                status: CommandStatus::Applied,
                ..Default::default()
            }],
            timestamp: 123456,
            ..Default::default()
        };

        let block_payload = CanonicalMainnetBlockPayload {
            block: test_block,
            canonical: false,
            was_canonical: true, // simulating a "rollback" of something never truly stored
        };

        // 4) Send this non-canonical block
        sender
            .send(Event {
                event_type: EventType::CanonicalMainnetBlock,
                payload: sonic_rs::to_string(&block_payload).unwrap(),
            })
            .await
            .expect("Failed to send non-canonical referencing an undiscovered account");

        // 5) Wait a bit for processing
        tokio::time::sleep(Duration::from_millis(200)).await;

        // 6) Check the sink => we expect NO `NewAccount` events at all
        let dag_locked = dag.lock().await;
        let captured_payloads = read_captured_new_accounts(&dag_locked, &sink_node_id).await;
        assert!(
            captured_payloads.is_empty(),
            "No 'NewAccount' event should be emitted if the account wasn't discovered at that height."
        );

        // 7) Confirm the *preexisting* account remains in the DB at height=0
        {
            let client = connect_to_db().await;
            let check_sql = "SELECT height FROM discovered_accounts_store WHERE key = $1";
            let row_opt = client
                .query_opt(check_sql, &[&"B62qPhantomAccount"])
                .await
                .expect("Query failed against discovered_accounts_store");
            assert!(row_opt.is_some(), "Preexisting account should remain in the store after non-canonical block");
            let row = row_opt.unwrap();
            let final_height: i64 = row.get("height");
            assert_eq!(final_height, 0, "Expected the preexisting account to remain at height=0, not removed");
        }
    }

    /// Verifies that when we have a stored account at a particular height + state_hash,
    /// and we receive a non-canonical block with the **same** height and state_hash,
    /// the code removes the account from the store and emits a "NewAccount" event with `apply=false`.
    #[tokio::test]
    async fn test_revert_account_at_same_height_and_hash() {
        // 1) Build the DAG + root actor
        let mut dag = ActorDAG::new();
        let actor_node = NewAccountActor::create_actor(false).await; // `preserve_data=false`
        let actor_id = actor_node.id();
        let sender = dag.set_root(actor_node);

        // 2) Create sink node to capture `NewAccount` events, link from our actor
        let sink_node = create_new_account_sink_node();
        let sink_node_id = sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, &sink_node_id);

        // 3) Wrap + spawn the DAG
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // --------------------------------------------------------------------
        // 4) STEP A: Insert a canonical block that discovers an account at (height=42, state_hash="test_state")
        // --------------------------------------------------------------------

        // This block *will* discover "B62qRevertableAcct" => store (height=42, state_hash="test_state")
        // with `apply=true`.
        let discovered_account = "B62qRevertableAcct".to_string();

        let canonical_block = CanonicalMainnetBlockPayload {
            block: MainnetBlockPayload {
                height: 42,
                state_hash: "test_state".into(),
                user_commands: vec![CommandSummary {
                    sender: discovered_account.clone(),
                    receiver: discovered_account.clone(),
                    status: CommandStatus::Applied,
                    ..Default::default()
                }],
                timestamp: 111111,
                ..Default::default()
            },
            canonical: true,
            was_canonical: false,
        };

        sender
            .send(Event {
                event_type: EventType::CanonicalMainnetBlock,
                payload: sonic_rs::to_string(&canonical_block).unwrap(),
            })
            .await
            .expect("Failed to send canonical block event #1");

        // Wait for processing
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // Confirm the store has the account at height=42 + state_hash="test_state"
        {
            let client = connect_to_db().await;
            let sql = "SELECT height, state_hash FROM discovered_accounts_store WHERE key = $1";
            let row_opt = client.query_opt(sql, &[&discovered_account]).await.expect("Query failed");
            assert!(row_opt.is_some(), "Account should be discovered after the canonical block");
            let row = row_opt.unwrap();
            let db_height: i64 = row.get("height");
            let db_state: String = row.get("state_hash");
            assert_eq!(db_height, 42, "Expected height=42 in the store");
            assert_eq!(db_state, "test_state", "Expected state_hash='test_state' in the store");
        }

        // --------------------------------------------------------------------
        // 5) STEP B: Send a non-canonical block with the SAME (height=42, state_hash="test_state"). This should hit the revert scenario => remove the account
        //    => emit apply=false
        // --------------------------------------------------------------------
        let revert_block = CanonicalMainnetBlockPayload {
            block: MainnetBlockPayload {
                height: 42,
                state_hash: "test_state".into(),
                user_commands: vec![CommandSummary {
                    sender: discovered_account.clone(),
                    receiver: discovered_account.clone(),
                    status: CommandStatus::Applied,
                    ..Default::default()
                }],
                timestamp: 222222,
                ..Default::default()
            },
            canonical: false,    // reversing
            was_canonical: true, // it was previously canonical
        };

        sender
            .send(Event {
                event_type: EventType::CanonicalMainnetBlock,
                payload: sonic_rs::to_string(&revert_block).unwrap(),
            })
            .await
            .expect("Failed to send revert block event");

        // Wait for processing
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // --------------------------------------------------------------------
        // 6) Verify the actor emitted `NewAccount` with `apply=false`, and the store no longer has the account
        // --------------------------------------------------------------------
        let dag_locked = dag.lock().await;
        let captured_events = read_captured_new_accounts(&dag_locked, &sink_node_id).await;

        // We expect 2 total events for `discovered_account`:
        //   1) the canonical block with apply=true
        //   2) the revert block with apply=false
        // If you want to separate them out or check only the second, do so.
        let relevant: Vec<NewAccountPayload> = captured_events
            .iter()
            .filter_map(|json_str| sonic_rs::from_str::<NewAccountPayload>(json_str).ok())
            .filter(|nap| nap.account == discovered_account)
            .collect();

        assert_eq!(
            relevant.len(),
            2,
            "Expected two events for the same account: discovered (apply=true) and reverted (apply=false)."
        );

        // The second event must be `apply=false`
        let second_event = &relevant[1];
        assert!(!second_event.apply, "Second event must have apply=false");
        assert_eq!(second_event.height, 42, "Still referencing the same height=42");
        assert_eq!(second_event.state_hash, "test_state", "Still referencing the same state_hash='test_state'");

        // Confirm the store no longer has the account
        {
            let client = connect_to_db().await;
            let sql = "SELECT EXISTS(SELECT 1 FROM discovered_accounts_store WHERE key=$1)";
            let row_opt = client.query_opt(sql, &[&discovered_account]).await.expect("Query failed");
            // row_opt might be Some(row), but the boolean should be false
            if let Some(row) = row_opt {
                let exists: bool = row.get(0);
                assert!(!exists, "The account should have been removed from the store after reverting");
            }
        }
    }
}
