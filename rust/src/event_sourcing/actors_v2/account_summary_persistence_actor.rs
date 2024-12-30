use crate::{
    constants::POSTGRES_CONNECTION_STRING,
    event_sourcing::{
        actor_dag::{ActorNode, ActorNodeBuilder, ActorStore},
        events::EventType,
        managed_store::ManagedStore,          // <-- Your specialized key/value store
        payloads::AccountBalanceDeltaPayload, // your struct with { balance_deltas: HashMap<String, i64> }
    },
};
use log::error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::NoTls;

/// The key under which we store our `ManagedStore<Client>` (or whichever type you use).
const ACCOUNT_STORE_KEY: &str = "account_summary_store";

pub struct AccountSummaryPersistenceActor;

impl AccountSummaryPersistenceActor {
    /// Handles an `AccountLogBalanceDelta` event by parsing the payload and upserting
    /// each account’s delta into the store.
    async fn handle_account_delta(payload: AccountBalanceDeltaPayload, actor_store: Arc<Mutex<ActorStore>>) {
        let mut store = { actor_store.lock().await };
        // Retrieve the store from the ActorStore
        let managed_store = store.remove::<ManagedStore>(ACCOUNT_STORE_KEY).expect("Missing ManagedStore in the ActorStore");

        // For each (account, delta) pair
        for (account, delta) in payload.balance_deltas {
            if delta == 0 {
                continue; // No change
            }

            if let Err(e) = managed_store.incr(&account, "balance", delta).await {
                error!("Failed to upsert balance for account={}: {}", account, e);
            }
        }

        store.insert::<ManagedStore>(ACCOUNT_STORE_KEY, managed_store);
    }

    pub async fn create_actor(preserve_data: bool) -> ActorNode {
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

        // 4) Build the actor node with the event processor
        ActorNodeBuilder::new()
            .with_state(actor_store)
            .with_processor(|event, actor_store, _requeue| {
                Box::pin(async move {
                    // Only handle `AccountLogBalanceDelta`
                    if event.event_type == EventType::AccountLogBalanceDelta {
                        // parse the payload
                        let payload: AccountBalanceDeltaPayload = sonic_rs::from_str(&event.payload).expect("Failed to parse AccountBalanceDeltaPayload");

                        AccountSummaryPersistenceActor::handle_account_delta(payload, actor_store).await;
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
        constants::POSTGRES_CONNECTION_STRING,
        event_sourcing::{
            actor_dag::ActorDAG,
            events::{Event, EventType},
            payloads::AccountBalanceDeltaPayload,
        },
    };
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tokio_postgres::NoTls;

    /// Utility to fetch the numeric `balance` from `account_summary_store`
    /// for a given `account`.
    async fn fetch_account_balance(client: &tokio_postgres::Client, account: &str) -> i64 {
        let sql = "SELECT balance FROM account_summary_store WHERE key = $1";
        let row_opt = client.query_opt(sql, &[&account]).await.expect("Failed to query balance");
        row_opt.map(|row| row.get::<_, i64>("balance")).unwrap_or(0)
    }

    #[tokio::test]
    async fn test_account_summary_persistence_actor_multiple_increments() {
        // 1) Build the ActorDAG
        let mut dag = ActorDAG::new();

        // 2) Create the actor node
        let actor_node = AccountSummaryPersistenceActor::create_actor(false).await;

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

        // 5) First event: increment acct1 => +100, acct2 => -50
        use std::collections::HashMap;
        let mut deltas_1 = HashMap::new();
        deltas_1.insert("acct1".to_string(), 100i64);
        deltas_1.insert("acct2".to_string(), -50i64);

        let payload_1 = AccountBalanceDeltaPayload { balance_deltas: deltas_1 };

        actor_sender
            .send(Event {
                event_type: EventType::AccountLogBalanceDelta,
                payload: sonic_rs::to_string(&payload_1).unwrap(),
            })
            .await
            .expect("Failed to send first AccountLogBalanceDelta event");

        // 6) Second event: increment acct1 => +25, acct2 => +75
        let mut deltas_2 = HashMap::new();
        deltas_2.insert("acct1".to_string(), 25i64);
        deltas_2.insert("acct2".to_string(), 75i64);

        let payload_2 = AccountBalanceDeltaPayload { balance_deltas: deltas_2 };

        actor_sender
            .send(Event {
                event_type: EventType::AccountLogBalanceDelta,
                payload: sonic_rs::to_string(&payload_2).unwrap(),
            })
            .await
            .expect("Failed to send second AccountLogBalanceDelta event");

        // Give it some processing time
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

        let acct1_balance = fetch_account_balance(&client, "acct1").await;
        let acct2_balance = fetch_account_balance(&client, "acct2").await;

        // 8) Confirm final balances
        // For acct1: first event +100, second event +25 => total +125
        // For acct2: first event -50, second event +75 => total +25
        assert_eq!(acct1_balance, 125, "acct1 should have +125 total");
        assert_eq!(acct2_balance, 25, "acct2 should have +25 total");
    }
}
