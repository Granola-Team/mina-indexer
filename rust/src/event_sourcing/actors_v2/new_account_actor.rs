use crate::{
    constants::POSTGRES_CONNECTION_STRING,
    event_sourcing::{
        actor_dag::{ActorNode, ActorNodeBuilder, ActorStore},
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
            if let Ok(true) = managed_store.key_exists(acct).await {
                if !block.canonical {
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
                } else {
                    debug!("Account already discovered {acct}");
                }
            } else if block.canonical {
                let pairs = &[("height", &(block.block.height as i64) as &(dyn tokio_postgres::types::ToSql + Sync))];
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
            payloads::{CanonicalMainnetBlockPayload /* used instead of MainnetBlockPayload + confirmations */, NewAccountPayload},
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
    async fn test_existing_account_mixed_canonical_blocks() {
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

        // 1) Insert a "preexisting" account
        let existing_account = "B62qExisting1".to_string();
        sender
            .send(Event {
                event_type: EventType::PreExistingAccount,
                payload: existing_account.clone(),
            })
            .await
            .unwrap();

        // 2) Build two blocks referencing the same account:
        //    - BlockA => canonical = true
        //    - BlockB => canonical = false (like a rollback scenario)
        let block_a = CanonicalMainnetBlockPayload {
            block: crate::event_sourcing::payloads::MainnetBlockPayload {
                height: 11,
                state_hash: "hash_A".into(),
                user_commands: vec![CommandSummary {
                    sender: existing_account.clone(),
                    receiver: existing_account.clone(),
                    fee_payer: existing_account.clone(),
                    status: CommandStatus::Applied,
                    ..Default::default()
                }],
                timestamp: 999111,
                ..Default::default()
            },
            canonical: true,
            was_canonical: false,
        };
        let block_b = CanonicalMainnetBlockPayload {
            block: crate::event_sourcing::payloads::MainnetBlockPayload {
                height: 12,
                state_hash: "hash_B".into(),
                user_commands: vec![CommandSummary {
                    sender: existing_account.clone(),
                    receiver: existing_account.clone(),
                    fee_payer: existing_account.clone(),
                    status: CommandStatus::Applied,
                    ..Default::default()
                }],
                timestamp: 999222,
                ..Default::default()
            },
            canonical: false,
            was_canonical: true, // This scenario is "unapplying" an old canonical block
        };

        // 3) Send blockA => canonical => referencing existing account => no new account event
        sender
            .send(Event {
                event_type: EventType::CanonicalMainnetBlock,
                payload: sonic_rs::to_string(&block_a).unwrap(),
            })
            .await
            .unwrap();

        // 4) Send blockB => non-canonical => referencing same account => "remove" scenario
        sender
            .send(Event {
                event_type: EventType::CanonicalMainnetBlock,
                payload: sonic_rs::to_string(&block_b).unwrap(),
            })
            .await
            .unwrap();

        // Wait
        sleep(Duration::from_millis(400)).await;

        // 5) Check the sink => we expect exactly 1 event from blockB (the “apply=false” scenario), but only if the account *had not* previously been
        //    discovered. However, we inserted it as "preexisting", so let's see if the code tries to remove it.
        let dag_locked = dag.lock().await;
        let captured = read_captured_new_accounts(&dag_locked, &sink_id).await;

        // Because the account was preexisting, blockA (canonical) does not produce a new account
        // blockB (non-canonical) might produce "apply=false" if it tries to remove a discovered account
        // ... or possibly no event if your code checks for "existing" first.
        // Adjust the assertions to match your intended logic. For example:
        assert_eq!(
            captured.len(),
            1,
            "We expect a single 'apply=false' event removing the existing account (or 0 if your logic differs)."
        );
        let payload_json = &captured[0];
        let new_account_payload: NewAccountPayload = sonic_rs::from_str(payload_json).unwrap();
        assert_eq!(new_account_payload.apply, false, "Should be a 'removal' event");
        assert_eq!(new_account_payload.account, existing_account, "It's the same account we had");
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
}
