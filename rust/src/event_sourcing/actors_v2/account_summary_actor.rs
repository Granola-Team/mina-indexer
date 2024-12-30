use crate::event_sourcing::{
    actor_dag::{ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
    events::{Event, EventType},
    payloads::{AccountBalanceDeltaPayload, AccountingEntry, AccountingEntryAccountType, AccountingEntryType, DoubleEntryRecordPayload, LedgerDestination},
};
use itertools::Itertools;
use std::collections::HashMap;

/// Name your actor something like `AccountSummaryActor`.
pub struct AccountSummaryActor;

impl AccountSummaryActor {
    /// Processes a `DoubleEntryTransaction` event, sums up partial changes for each account,
    /// and emits `AccountLogBalanceDelta` events, one per account (if the net delta != 0).
    async fn handle_transaction(event: Event) -> Option<Vec<Event>> {
        // 1) Parse the `DoubleEntryRecordPayload`
        let record: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).expect("Failed to parse DoubleEntryRecordPayload");

        // 2) We only process records that target `LedgerDestination::BlockchainLedger`
        if record.ledger_destination != LedgerDestination::BlockchainLedger {
            return None;
        }

        // 3) Interleave LHS + RHS into a single list of entries
        let combined_entries: Vec<AccountingEntry> = record
            .lhs
            .iter()
            .interleave(record.rhs.iter())
            .cloned()
            .filter(|r| r.account_type == AccountingEntryAccountType::BlockchainAddress)
            .collect();

        // 4) Build a local aggregator: per-account => net balance_delta (signed).
        let mut account_deltas: HashMap<String, i64> = HashMap::new();

        for entry in combined_entries {
            let delta = match entry.entry_type {
                AccountingEntryType::Credit => entry.amount_nanomina as i64,
                AccountingEntryType::Debit => -(entry.amount_nanomina as i64),
            };

            // Accumulate in the map
            *account_deltas.entry(entry.account.clone()).or_insert(0) += delta;
        }

        let out_events = vec![Event {
            event_type: EventType::AccountLogBalanceDelta,
            payload: sonic_rs::to_string(&AccountBalanceDeltaPayload {
                balance_deltas: account_deltas,
            })
            .unwrap(),
        }];

        // If no deltas, return None => no events
        if out_events.is_empty() {
            None
        } else {
            Some(out_events)
        }
    }
}

#[async_trait::async_trait]
impl ActorFactory for AccountSummaryActor {
    /// Build an actor node that:
    ///   - has no special state in the `ActorStore`,
    ///   - reacts to `DoubleEntryTransaction` events,
    ///   - emits `AccountLogBalanceDelta` events with the aggregated account deltas.
    async fn create_actor() -> ActorNode {
        // 1) Create an empty store
        let store = ActorStore::new();

        // 2) Build the actor node
        ActorNodeBuilder::new()
            .with_state(store)
            .with_processor(|event, _store, _requeue| {
                Box::pin(async move {
                    // Only handle DoubleEntryTransaction
                    if event.event_type == EventType::DoubleEntryTransaction {
                        AccountSummaryActor::handle_transaction(event).await
                    } else {
                        None
                    }
                })
            })
            .build()
    }
}

#[cfg(test)]
mod account_summary_actor_tests_v2 {
    use super::AccountSummaryActor;
    use crate::event_sourcing::{
        actor_dag::{ActorDAG, ActorFactory, ActorNode},
        events::{Event, EventType},
        payloads::{AccountBalanceDeltaPayload, AccountingEntry, AccountingEntryAccountType, AccountingEntryType, DoubleEntryRecordPayload, LedgerDestination},
    };
    use std::sync::Arc;
    use tokio::sync::Mutex;

    /// Helper: sets up an ActorDAG with the `AccountSummaryActor` as root.
    /// Returns the DAG, plus a `Sender<Event>` for us to send events into the actor.
    async fn setup_actor_dag() -> (Arc<Mutex<ActorDAG>>, tokio::sync::mpsc::Sender<Event>, String) {
        // 1) Create the actor via its factory
        let actor_node: ActorNode = AccountSummaryActor::create_actor().await;
        let actor_node_id = actor_node.id();

        // 2) Build a new ActorDAG and set the root
        let mut dag = ActorDAG::new();
        let sender = dag.set_root(actor_node);

        // 3) Wrap in Arc<Mutex<>> so we can spawn
        let dag = Arc::new(Mutex::new(dag));
        // 4) Spawn the DAG
        {
            let dag_clone = Arc::clone(&dag);
            tokio::spawn(async move {
                dag_clone.lock().await.spawn_all().await;
            });
        }

        (dag, sender, actor_node_id)
    }

    /// This sink node captures events of type `AccountLogBalanceDelta`
    /// so we can verify the aggregated results.
    fn create_balance_delta_sink() -> ActorNode {
        use crate::event_sourcing::{actor_dag::ActorNodeBuilder, events::EventType};

        ActorNodeBuilder::new()
            .with_state(Default::default())
            .with_processor(|event, state, _requeue| {
                Box::pin(async move {
                    if event.event_type == EventType::AccountLogBalanceDelta {
                        // Store the JSON in a vector named "balance_delta_events"
                        let mut locked_state = state.lock().await;
                        let mut captured: Vec<String> = locked_state.get("balance_delta_events").cloned().unwrap_or_default();
                        captured.push(event.payload.clone());
                        locked_state.insert("balance_delta_events", captured);
                    }
                    None
                })
            })
            .build()
    }

    /// Reads out the entire vector of captured balance-delta payloads (as JSON strings).
    async fn read_captured_balance_deltas(dag: &Arc<Mutex<ActorDAG>>, sink_id: &str) -> Vec<String> {
        let dag_locked = dag.lock().await;
        let node = dag_locked.read_node(sink_id.to_string()).expect("Sink node not found");
        let state = node.lock().await.get_state();
        let locked_store = state.lock().await;

        locked_store.get::<Vec<String>>("balance_delta_events").cloned().unwrap_or_default()
    }

    #[tokio::test]
    async fn test_actor_aggregates_multiple_accounts() {
        // 1) Setup the actor
        let (dag, actor_sender, root_node_id) = setup_actor_dag().await;

        // 2) Create a sink node to capture `AccountLogBalanceDelta` events
        let sink_node = create_balance_delta_sink();
        let sink_node_id = sink_node.id();

        // 3) Add the sink node to the DAG, link from the actor's root
        {
            let mut dag_locked = dag.lock().await;
            dag_locked.add_node(sink_node);

            // Link the root => sink
            dag_locked.link_parent(&root_node_id, &sink_node_id);
        }

        // 4) Build a DoubleEntryRecordPayload that has multiple repeated accounts
        // in both LHS and RHS to confirm net summation.
        let payload = DoubleEntryRecordPayload {
            height: 999,
            state_hash: "test_state_hash".to_string(),
            ledger_destination: LedgerDestination::BlockchainLedger,
            lhs: vec![
                // LHS #1
                AccountingEntry {
                    entry_type: AccountingEntryType::Debit,
                    account: "acct1".to_string(),
                    account_type: AccountingEntryAccountType::BlockchainAddress,
                    amount_nanomina: 1000,
                    timestamp: 123456789,
                    counterparty: "cp1".to_string(),
                    transfer_type: "t1".to_string(),
                },
                // LHS #2
                AccountingEntry {
                    entry_type: AccountingEntryType::Credit,
                    account: "acct2".to_string(),
                    account_type: AccountingEntryAccountType::BlockchainAddress,
                    amount_nanomina: 500,
                    timestamp: 123456790,
                    counterparty: "cp2".to_string(),
                    transfer_type: "t2".to_string(),
                },
                // LHS #3: repeated acct1
                AccountingEntry {
                    entry_type: AccountingEntryType::Credit,
                    account: "acct1".to_string(),
                    account_type: AccountingEntryAccountType::BlockchainAddress,
                    amount_nanomina: 200,
                    timestamp: 123456791,
                    counterparty: "cp3".to_string(),
                    transfer_type: "t3".to_string(),
                },
            ],
            rhs: vec![
                // RHS #1
                AccountingEntry {
                    entry_type: AccountingEntryType::Debit,
                    account: "acct2".to_string(),
                    account_type: AccountingEntryAccountType::BlockchainAddress,
                    amount_nanomina: 100,
                    timestamp: 123456792,
                    counterparty: "cp4".to_string(),
                    transfer_type: "t4".to_string(),
                },
                // RHS #2
                AccountingEntry {
                    entry_type: AccountingEntryType::Credit,
                    account: "acct3".to_string(),
                    account_type: AccountingEntryAccountType::BlockchainAddress,
                    amount_nanomina: 999,
                    timestamp: 123456793,
                    counterparty: "cp5".to_string(),
                    transfer_type: "t5".to_string(),
                },
            ],
        };

        // 5) Send the DoubleEntryTransaction event
        actor_sender
            .send(Event {
                event_type: EventType::DoubleEntryTransaction,
                payload: sonic_rs::to_string(&payload).unwrap(),
            })
            .await
            .expect("Failed to send event");

        // 6) Wait some ms for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        // 7) Read from the sink node => we expect 1 event of type AccountLogBalanceDelta
        let raw_captured_events = read_captured_balance_deltas(&dag, &sink_node_id).await;
        assert_eq!(raw_captured_events.len(), 1, "Expected exactly one output event from the aggregator");

        // 8) Parse the single event's payload
        let event_json = &raw_captured_events[0];
        let parsed_payload: AccountBalanceDeltaPayload = sonic_rs::from_str(event_json).expect("Failed to parse AccountBalanceDeltaPayload");

        // 9) Confirm the net deltas
        // Let's see how the sums happen:
        //   For acct1 => LHS: Debit(1000) => -1000, Credit(200) => +200 => net -800
        //   For acct2 => LHS: Credit(500) => +500, RHS: Debit(100) => -100 => net +400
        //   For acct3 => RHS: Credit(999) => +999
        // Summaries =>
        //   acct1 => -800
        //   acct2 => +400
        //   acct3 => +999
        let map = &parsed_payload.balance_deltas;
        assert_eq!(map.len(), 3, "Expected 3 accounts in the aggregator");
        assert_eq!(map.get("acct1").copied(), Some(-800), "acct1 net delta should be -800");
        assert_eq!(map.get("acct2").copied(), Some(400), "acct2 net delta should be +400");
        assert_eq!(map.get("acct3").copied(), Some(999), "acct3 net delta should be +999");
    }
}
