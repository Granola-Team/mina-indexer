use crate::{
    constants::POSTGRES_CONNECTION_STRING,
    event_sourcing::{
        actor_dag::{ActorNode, ActorNodeBuilder, ActorStore},
        events::{Event, EventType},
        managed_store::ManagedStore,
    },
    // any other imports you need
};
use log::{debug, error};
use tokio_postgres::NoTls;

/// We'll store file paths in a simple "pcb_filter_store" with just a key TEXT PRIMARY KEY
/// + a dummy text column for illustration (we won't necessarily use it).
const PCB_FILTER_STORE: &str = "pcb_filter_store";

/// Our new PcbFilterActor
pub struct PcbFilterActor;

impl PcbFilterActor {
    pub async fn create_actor(preserve_data: bool) -> ActorNode {
        // 1) Connect to Postgres
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Failed to connect to the database for PcbFilterActor");
        // Spawn the connection task
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("Postgres connection error in PcbFilterActor: {}", e);
            }
        });

        // 2) Build the ManagedStore. We'll store file paths as the 'key', plus one text column if you like. You can keep an empty column set if desired.
        let store_builder = ManagedStore::builder(client)
            .name(PCB_FILTER_STORE) // table name in DB
            .add_text_column("processed"); // example column

        // If you want to preserve data across runs, you can do `preserve_data()`
        let managed_store = if preserve_data {
            store_builder
                .preserve_data()
                .build()
                .await
                .expect("Failed to build ManagedStore for PcbFilterActor")
        } else {
            store_builder.build().await.expect("Failed to build ManagedStore for PcbFilterActor")
        };

        // 3) Put the store into the ActorStore
        let mut actor_store = ActorStore::new();
        actor_store.insert(PCB_FILTER_STORE, managed_store);

        // 4) Build the actor node with an event processor
        ActorNodeBuilder::new()
            .with_state(actor_store)
            .with_processor(|event, state, _requeue| {
                Box::pin(async move {
                    let mut locked_state = state.lock().await;
                    let managed_store = locked_state
                        .remove::<ManagedStore>(PCB_FILTER_STORE)
                        .expect("Missing ManagedStore in PcbFilterActor state");

                    let file_path = event.payload.clone();
                    let exists = match managed_store.key_exists(&file_path).await {
                        Ok(val) => val,
                        Err(err) => {
                            error!("Error checking if file path exists: {err}");
                            locked_state.insert(PCB_FILTER_STORE, managed_store);
                            return None; // skip if DB error
                        }
                    };

                    let maybe_out_events = if !exists {
                        // Insert (key= file_path, no columns => upsert with defaults)
                        if let Err(e) = managed_store.upsert(&file_path, &[]).await {
                            error!("Failed to upsert file_path={file_path}: {e}");
                            // no event on error
                            None
                        } else {
                            debug!("PcbFilterActor discovered new file path: {file_path}");
                            // produce an event with event_type = PrecomputedBlockPath
                            Some(vec![Event {
                                event_type: EventType::PrecomputedBlockPath,
                                payload: file_path,
                            }])
                        }
                    } else {
                        debug!("File path already seen => ignoring: {file_path}");
                        None
                    };

                    // 4d) Put the store back
                    locked_state.insert(PCB_FILTER_STORE, managed_store);

                    maybe_out_events
                })
            })
            .build()
    }
}

#[cfg(test)]
mod pcb_filter_actor_tests {
    use super::PcbFilterActor;
    use crate::{
        // If you have a constant or a function for POSTGRES_CONNECTION_STRING, import it:
        constants::POSTGRES_CONNECTION_STRING,
        event_sourcing::{
            actor_dag::{ActorDAG, ActorNode, ActorNodeBuilder, ActorStore},
            events::{Event, EventType},
        },
    };
    use std::sync::Arc;
    use tokio::{
        sync::Mutex,
        time::{sleep, Duration},
    };
    use tokio_postgres::NoTls;

    /// We'll define a sink node that captures any `PrecomputedBlockPath` event
    /// and stores the payload(s) in a vector under "captured_paths".
    fn create_sink_node() -> ActorNode {
        ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|incoming_event, state, _requeue| {
                Box::pin(async move {
                    if incoming_event.event_type == EventType::PrecomputedBlockPath {
                        let mut locked_store = state.lock().await;
                        let mut captured_paths: Vec<String> = locked_store.get("captured_paths").cloned().unwrap_or_default();
                        captured_paths.push(incoming_event.payload.clone());
                        locked_store.insert("captured_paths", captured_paths);
                    }
                    None
                })
            })
            .build()
    }

    /// Helper to read captured `PrecomputedBlockPath` events from the sink node
    async fn read_captured_paths(dag: &ActorDAG, sink_node_id: &str) -> Vec<String> {
        let node_arc = dag.read_node(sink_node_id.to_string()).expect("Sink node not found");
        let node_guard = node_arc.lock().await; // lock the node
        let store = node_guard.get_state();
        let locked_store = store.lock().await;
        locked_store.get::<Vec<String>>("captured_paths").cloned().unwrap_or_default()
    }

    /// Helper to connect to DB, so we can query the table "pcb_filter_store" to confirm file paths
    async fn connect_db() -> tokio_postgres::Client {
        let (client, conn) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Failed to connect to Postgres in test");

        tokio::spawn(async move {
            if let Err(e) = conn.await {
                eprintln!("connection error in test: {}", e);
            }
        });

        client
    }

    #[tokio::test]
    async fn test_pcb_filter_actor_in_dag() {
        // 1) Create the DAG
        let mut dag = ActorDAG::new();

        // 2) Create the PcbFilterActor with preserve_data=false
        let pcb_actor_node = PcbFilterActor::create_actor(false).await;
        let pcb_actor_id = pcb_actor_node.id();

        // 3) Add it as root => get a sender
        let pcb_actor_sender = dag.set_root(pcb_actor_node);

        // 4) Create a sink node
        let sink_node = create_sink_node();
        let sink_node_id = sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&pcb_actor_id, &sink_node_id);

        // 5) Wrap + spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 6) We'll send two unique file paths + a duplicate to test behavior
        let file_path_a = "/path/to/fileA.json".to_string();
        let file_path_b = "/path/to/fileB.json".to_string();

        // 7) Send the first file path
        pcb_actor_sender
            .send(Event {
                event_type: EventType::PrecomputedBlockPath, // or anything your system uses; the actor only checks payload
                payload: file_path_a.clone(),
            })
            .await
            .expect("Failed to send file_path_a event");

        // 8) Wait and read sink
        sleep(Duration::from_millis(200)).await;
        {
            let dag_locked = dag.lock().await;
            let captured = read_captured_paths(&dag_locked, &sink_node_id).await;
            assert_eq!(captured.len(), 1, "Expected exactly 1 captured path (file_path_a) so far");
            assert_eq!(captured[0], file_path_a);
        }

        // 9) Send the same file path again => we expect NO new event
        pcb_actor_sender
            .send(Event {
                event_type: EventType::PrecomputedBlockPath,
                payload: file_path_a.clone(),
            })
            .await
            .expect("Failed to send file_path_a again");
        sleep(Duration::from_millis(200)).await;
        {
            let dag_locked = dag.lock().await;
            let captured = read_captured_paths(&dag_locked, &sink_node_id).await;
            assert_eq!(captured.len(), 1, "Still only 1 path, because file_path_a is a duplicate");
        }

        // 10) Send a new file path => we expect 1 more event
        pcb_actor_sender
            .send(Event {
                event_type: EventType::PrecomputedBlockPath,
                payload: file_path_b.clone(),
            })
            .await
            .expect("Failed to send file_path_b");
        sleep(Duration::from_millis(200)).await;
        {
            let dag_locked = dag.lock().await;
            let captured = read_captured_paths(&dag_locked, &sink_node_id).await;
            assert_eq!(captured.len(), 2, "Now we should have exactly 2 distinct captures: file_path_a and file_path_b");
            assert_eq!(captured[1], file_path_b, "The second path captured is file_path_b");
        }

        // 11) Finally, let's confirm that both fileA and fileB are present in the 'pcb_filter_store' table using the Postgres client.
        let client = connect_db().await;
        let query = "SELECT key FROM pcb_filter_store ORDER BY key";
        let rows = client.query(query, &[]).await.expect("Failed to query pcb_filter_store");
        let keys: Vec<String> = rows.iter().map(|r| r.get("key")).collect();
        assert_eq!(keys.len(), 2, "Expected exactly 2 keys in pcb_filter_store");
        assert!(keys.contains(&file_path_a), "Should contain file_path_a");
        assert!(keys.contains(&file_path_b), "Should contain file_path_b");
    }
}
