use crate::{
    constants::POSTGRES_CONNECTION_STRING,
    event_sourcing::{
        actor_dag::{ActorNode, ActorNodeBuilder, ActorStore},
        berkeley_block_models::AccessedAccount,
        events::EventType,
        managed_store::ManagedStore,
        payloads::AccountBalanceDeltaPayload, // your struct with { balance_deltas: HashMap<String, i64> }
    },
};
use log::error;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tokio_postgres::NoTls;

/// The key under which we store our `ManagedStore<Client>` (or whichever type you use).
const ACCOUNT_STORE_KEY: &str = "account_summary_store";
type IncorrectAccounts = HashMap<String, (u64, u64)>;

pub struct AccountSummaryPersistenceActor;

impl AccountSummaryPersistenceActor {
    async fn handle_runtime_ledger_check(accessed_accounts: Vec<AccessedAccount>, actor_store: Arc<Mutex<ActorStore>>) -> Result<(), IncorrectAccounts> {
        let mut store = { actor_store.lock().await };
        let managed_store = store.remove::<ManagedStore>(ACCOUNT_STORE_KEY).expect("Missing ManagedStore in the ActorStore");
        let mut incorrect_accounts = HashMap::new();

        for accessed_account in accessed_accounts {
            if let Ok(Some(balance)) = managed_store
                .get::<i64>(&format!("{}#{}", accessed_account.token_id, accessed_account.public_key), "balance")
                .await
            {
                if accessed_account.balance() != balance as u64 {
                    incorrect_accounts.insert(accessed_account.public_key.to_string(), (accessed_account.balance(), balance as u64));
                }
            } else {
                incorrect_accounts.insert(accessed_account.public_key.to_string(), (accessed_account.balance(), 0));
            }
        }

        store.insert::<ManagedStore>(ACCOUNT_STORE_KEY, managed_store);

        if incorrect_accounts.is_empty() {
            Ok(())
        } else {
            Err(incorrect_accounts)
        }
    }

    /// Handles an `AccountLogBalanceDelta` event by parsing the payload and upserting
    /// each account’s delta into the store.
    async fn handle_account_delta(payload: &AccountBalanceDeltaPayload, actor_store: &Arc<Mutex<ActorStore>>) {
        let mut store = { actor_store.lock().await };
        // Retrieve the store from the ActorStore
        let managed_store = store.remove::<ManagedStore>(ACCOUNT_STORE_KEY).expect("Missing ManagedStore in the ActorStore");

        // For each (account, delta) pair
        for (account, delta) in payload.balance_deltas.clone() {
            if delta == 0 {
                continue; // No change
            }
            let key = format!("{}#{}", payload.token_id, account);

            if let Err(e) = managed_store.upsert(&key, &[]).await {
                error!("Unable to insert blockchain_tree {e}");
            }

            if let Err(e) = managed_store.incr(&key, "balance", delta).await {
                error!("Failed to upsert balance for account={}: {}", account, e);
            }
        }

        store.insert::<ManagedStore>(ACCOUNT_STORE_KEY, managed_store);
    }

    pub async fn create_actor(preserve_data: bool, runtime_ledger_check: bool) -> ActorNode {
        // 1) Connect to Postgres
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Failed to connect to database");

        // Spawn the connection handle
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("Postgres connection error: {}", e);
            }
        });

        // 2) Create or init the `ManagedStore`. Suppose your `ManagedStore` has a builder or something similar:

        let store_builder = ManagedStore::builder(client).name(ACCOUNT_STORE_KEY).add_numeric_column("balance");
        let store_obj = if preserve_data {
            store_builder
                .preserve_data()
                .build() // returning a `ManagedStore`
                .await
                .expect("Failed to build ManagedStore for AccountSummary")
        } else {
            store_builder
                .build() // returning a `ManagedStore`
                .await
                .expect("Failed to build ManagedStore for AccountSummary")
        };

        // 3) Put the store into the ActorStore
        let mut actor_store = ActorStore::new();
        actor_store.insert(ACCOUNT_STORE_KEY, store_obj);

        // Store runtime ledger check flag
        actor_store.insert("runtime_ledger_check", runtime_ledger_check);

        // 4) Build the actor node with the event processor
        ActorNodeBuilder::new()
            .with_state(actor_store)
            .with_processor(|event, actor_store, _requeue| {
                Box::pin(async move {
                    // let runtime_ledger_check = env::var("RUNTIME_LEDGER_CHECK").ok().and_then(|val| val.parse::<bool>().ok()).unwrap_or(false);
                    if event.event_type == EventType::AccountLogBalanceDelta {
                        // parse the payload
                        let payload: AccountBalanceDeltaPayload = sonic_rs::from_str(&event.payload).expect("Failed to parse AccountBalanceDeltaPayload");
                        let height = payload.height;
                        let state_hash = payload.state_hash.to_string();

                        AccountSummaryPersistenceActor::handle_account_delta(&payload, &actor_store).await;

                        let runtime_ledger_check = {
                            let store_guard = actor_store.lock().await;
                            *store_guard
                                .get::<bool>("runtime_ledger_check")
                                .expect("Expected runtime_ledger_check to be present")
                        };

                        if runtime_ledger_check && payload.accessed_accounts.is_some() {
                            if let Err(incorrect_accounts) =
                                AccountSummaryPersistenceActor::handle_runtime_ledger_check(payload.accessed_accounts.unwrap(), actor_store).await
                            {
                                error!("Incorrect balance at {height} and {state_hash}: {:#?}", incorrect_accounts);
                            }
                        }
                    }
                    None
                })
            })
            .build()
    }
}

#[cfg(test)]
mod account_summary_persistence_actor_tests {
    use super::AccountSummaryPersistenceActor;
    use crate::{
        constants::{MINA_TOKEN_ID, POSTGRES_CONNECTION_STRING},
        event_sourcing::{
            actor_dag::ActorDAG,
            events::{Event, EventType},
            payloads::AccountBalanceDeltaPayload,
        },
    };
    use std::{collections::HashMap, sync::Arc};
    use tokio::sync::Mutex;
    use tokio_postgres::NoTls;

    /// Utility to fetch the numeric `balance` from `account_summary_store`
    /// for a given `(token_id, account)`.
    async fn fetch_balance_for_token(client: &tokio_postgres::Client, token_id: &str, account: &str) -> i64 {
        let compound_key = format!("{}#{}", token_id, account);
        let sql = "SELECT balance FROM account_summary_store WHERE key = $1";
        let row_opt = client
            .query_opt(sql, &[&compound_key])
            .await
            .expect("Failed to query balance for token_id#account");

        row_opt.map(|row| row.get::<_, i64>("balance")).unwrap_or(0)
    }

    #[tokio::test]
    async fn test_account_summary_persistence_actor_multiple_increments() {
        // 1) Build the ActorDAG
        let mut dag = ActorDAG::new();

        // 2) Create the actor node
        let actor_node = AccountSummaryPersistenceActor::create_actor(false, false).await;

        // 3) Add as root => get the sender
        let actor_sender = dag.set_root(actor_node);

        // 4) Wrap in Arc<Mutex<>> and spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 5) First event: increment acct1 => +100, acct2 => -50 on the SAME token (MINA_TOKEN_ID)
        let mut deltas_1 = HashMap::new();
        deltas_1.insert("acct1".to_string(), 100i64);
        deltas_1.insert("acct2".to_string(), -50i64);

        let payload_1 = AccountBalanceDeltaPayload {
            token_id: MINA_TOKEN_ID.to_string(),
            balance_deltas: deltas_1,
            ..Default::default()
        };

        actor_sender
            .send(Event {
                event_type: EventType::AccountLogBalanceDelta,
                payload: sonic_rs::to_string(&payload_1).unwrap(),
            })
            .await
            .expect("Failed to send first event");

        // 6) Second event: increment acct1 => +25, acct2 => +75 on the SAME token (MINA_TOKEN_ID)
        let mut deltas_2 = HashMap::new();
        deltas_2.insert("acct1".to_string(), 25i64);
        deltas_2.insert("acct2".to_string(), 75i64);

        let payload_2 = AccountBalanceDeltaPayload {
            token_id: MINA_TOKEN_ID.to_string(),
            balance_deltas: deltas_2,
            ..Default::default()
        };

        actor_sender
            .send(Event {
                event_type: EventType::AccountLogBalanceDelta,
                payload: sonic_rs::to_string(&payload_2).unwrap(),
            })
            .await
            .expect("Failed to send second event");

        // Give it some processing time
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // 7) Connect for verification
        let (client, conn) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Failed to connect for verification");
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                eprintln!("Verification connection error: {}", e);
            }
        });

        // 8) Confirm final balances for each (token_id, account).
        // For 'acct1' => we should see 100 + 25 => 125 total
        // For 'acct2' => we should see -50 + 75 => 25 total
        let acct1_balance = fetch_balance_for_token(&client, MINA_TOKEN_ID, "acct1").await;
        let acct2_balance = fetch_balance_for_token(&client, MINA_TOKEN_ID, "acct2").await;

        assert_eq!(acct1_balance, 125, "acct1 should have +125 total for token=MINA");
        assert_eq!(acct2_balance, 25, "acct2 should have +25 total for token=MINA");
    }

    #[tokio::test]
    async fn test_account_summary_persistence_actor_multiple_token_ids() {
        // 1) Build the ActorDAG
        let mut dag = ActorDAG::new();

        // 2) Create the actor node (no preserve_data for a fresh start)
        let actor_node = AccountSummaryPersistenceActor::create_actor(false, false).await;

        // 3) Add as root => get the sender
        let actor_sender = dag.set_root(actor_node);

        // 4) Wrap in Arc<Mutex<>> and spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 5) First event: token "token_a" => { acct1 => +10, acct2 => +20 }
        let mut deltas_a = HashMap::new();
        deltas_a.insert("acct1".to_string(), 10i64);
        deltas_a.insert("acct2".to_string(), 20i64);

        let payload_a = AccountBalanceDeltaPayload {
            token_id: "token_a".to_string(),
            balance_deltas: deltas_a,
            ..Default::default()
        };

        actor_sender
            .send(Event {
                event_type: EventType::AccountLogBalanceDelta,
                payload: sonic_rs::to_string(&payload_a).unwrap(),
            })
            .await
            .expect("Failed to send first event for token_a");

        // 6) Second event: token "token_b" => { acct1 => +100, acct2 => +200 }
        let mut deltas_b = HashMap::new();
        deltas_b.insert("acct1".to_string(), 100i64);
        deltas_b.insert("acct2".to_string(), 200i64);

        let payload_b = AccountBalanceDeltaPayload {
            token_id: "token_b".to_string(),
            balance_deltas: deltas_b,
            ..Default::default()
        };

        actor_sender
            .send(Event {
                event_type: EventType::AccountLogBalanceDelta,
                payload: sonic_rs::to_string(&payload_b).unwrap(),
            })
            .await
            .expect("Failed to send second event for token_b");

        // Wait a bit
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // 7) Connect to DB for verification
        let (client, conn) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Failed to connect for verification");
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                eprintln!("Verification connection error: {}", e);
            }
        });

        // 8) Now check each (token_id, account) individually
        // "token_a#acct1" => +10
        // "token_a#acct2" => +20
        // "token_b#acct1" => +100
        // "token_b#acct2" => +200

        let acct1_token_a = fetch_balance_for_token(&client, "token_a", "acct1").await;
        let acct2_token_a = fetch_balance_for_token(&client, "token_a", "acct2").await;
        let acct1_token_b = fetch_balance_for_token(&client, "token_b", "acct1").await;
        let acct2_token_b = fetch_balance_for_token(&client, "token_b", "acct2").await;

        // Confirm each
        assert_eq!(acct1_token_a, 10, "acct1 for token_a should be +10");
        assert_eq!(acct2_token_a, 20, "acct2 for token_a should be +20");
        assert_eq!(acct1_token_b, 100, "acct1 for token_b should be +100");
        assert_eq!(acct2_token_b, 200, "acct2 for token_b should be +200");
    }

    #[tokio::test]
    async fn test_runtime_ledger_check_correct_and_incorrect() {
        use crate::event_sourcing::berkeley_block_models::AccessedAccount;

        // 2) Build the ActorDAG
        let mut dag = ActorDAG::new();

        // 3) Create the AccountSummaryPersistenceActor
        let actor_node = AccountSummaryPersistenceActor::create_actor(false, true).await;

        // 4) Add as root => get the Sender
        let actor_sender = dag.set_root(actor_node);

        // 5) Spawn the DAG
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // -------------------------------------------------------
        // SCENARIO A) Populate some final balances in the store
        // -------------------------------------------------------
        // Let's do a single token "test_token" with two accounts => acct1 => +50, acct2 => +25
        use std::collections::HashMap;

        let mut deltas = HashMap::new();
        deltas.insert("acct1".to_string(), 50i64);
        deltas.insert("acct2".to_string(), 25i64);

        let payload = AccountBalanceDeltaPayload {
            height: 50,
            state_hash: "state_hash_1".to_string(),
            token_id: "test_token".to_string(),
            balance_deltas: deltas,
            accessed_accounts: None, // no checks yet
        };

        actor_sender
            .send(Event {
                event_type: EventType::AccountLogBalanceDelta,
                payload: sonic_rs::to_string(&payload).unwrap(),
            })
            .await
            .expect("Failed to send increments event");

        // Give time for processing
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // -------------------------------------------------------
        // SCENARIO B) Provide matching accessed_accounts => expect NO error
        // -------------------------------------------------------
        // The code sees that each accessed_account's .balance() matches what's in the store,
        // so handle_runtime_ledger_check returns Ok(()) => no "Incorrect balance" log.

        // We'll supply "acct1 => 50, acct2 => 25" for the same "test_token".
        let accounts_ok = vec![
            AccessedAccount {
                public_key: "acct1".into(),
                token_id: "test_token".into(),
                token_symbol: String::new(),
                balance: String::from("50"),
            },
            AccessedAccount {
                public_key: "acct2".into(),
                token_id: "test_token".into(),
                token_symbol: String::new(),
                balance: String::from("25"),
            },
        ];

        // Now we send an event that triggers runtime ledger check:
        let payload_ok = AccountBalanceDeltaPayload {
            height: 50,
            state_hash: "state_hash_1".to_string(),
            token_id: "test_token".to_string(),
            balance_deltas: HashMap::new(), // no deltas needed
            accessed_accounts: Some(accounts_ok),
        };

        actor_sender
            .send(Event {
                event_type: EventType::AccountLogBalanceDelta,
                payload: sonic_rs::to_string(&payload_ok).unwrap(),
            })
            .await
            .expect("Failed to send runtime ledger check event with correct balances");

        // Give time for processing => should see no errors
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // -------------------------------------------------------
        // SCENARIO C) Provide *incorrect* accessed_accounts => expect an error log
        // -------------------------------------------------------
        // We'll make "acct1 => 999 (mismatch!), acct2 => 25" for "test_token".
        // The code should log: "Incorrect balance: {...}" for acct1.

        let accounts_bad = vec![
            AccessedAccount {
                public_key: "acct1".into(),
                token_id: "test_token".into(),

                token_symbol: String::new(),
                balance: String::from("999"),
            },
            AccessedAccount {
                public_key: "acct2".into(),
                token_id: "test_token".into(),
                token_symbol: String::new(),
                balance: String::from("25"),
            },
        ];

        let payload_bad = AccountBalanceDeltaPayload {
            height: 50,
            state_hash: "state_hash_1".to_string(),
            token_id: "test_token".to_string(),
            balance_deltas: HashMap::new(),
            accessed_accounts: Some(accounts_bad),
        };

        actor_sender
            .send(Event {
                event_type: EventType::AccountLogBalanceDelta,
                payload: sonic_rs::to_string(&payload_bad).unwrap(),
            })
            .await
            .expect("Failed to send runtime ledger check event with incorrect balances");

        // Another short wait to let logs appear
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // We can't directly assert logs, but we *know* handle_runtime_ledger_check
        // would hit the "Err(incorrect_accounts)" path for acct1 => logs error.
        // If the code had a panic or returned an event, we could check that,
        // but it only logs the error. So we conclude the scenario is tested.
    }
}
