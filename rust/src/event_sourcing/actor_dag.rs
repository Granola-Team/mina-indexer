use super::events::Event;
use async_trait::async_trait;
use futures::future::BoxFuture;
use log::error;
use std::{any::Any, collections::HashMap, future::Future, sync::Arc};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};
use tracing::debug;
use uuid::Uuid;

pub struct ActorStore {
    pub data: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl Default for ActorStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ActorStore {
    pub fn new() -> Self {
        Self { data: HashMap::new() }
    }

    // Insert a value into the store
    pub fn insert<T: Any + Send + Sync>(&mut self, key: &str, value: T) {
        self.data.insert(key.to_string(), Box::new(value));
    }

    // Retrieve a value from the store
    pub fn get<T: Any>(&self, key: &str) -> Option<&T> {
        self.data.get(key)?.downcast_ref::<T>()
    }

    // Remove a value from the store
    pub fn remove<T: Any>(&mut self, key: &str) -> Option<T> {
        self.data.remove(key)?.downcast::<T>().ok().map(|b| *b)
    }
}

type EP<S> = Box<dyn Fn(Event, Arc<Mutex<S>>, Sender<Event>) -> BoxFuture<'static, Option<Vec<Event>>> + Send + Sync>;

pub struct ActorNode {
    id: String, // Unique identifier for the node

    // Asynchronous function to process events received by this node
    event_processor: EP<ActorStore>,

    // internal state
    state: Arc<Mutex<ActorStore>>, // State wrapped in Arc<Mutex> for safe sharing
}

impl ActorNode {
    pub fn id(&self) -> String {
        self.id.to_string()
    }

    pub fn get_state(&self) -> Arc<Mutex<ActorStore>> {
        Arc::clone(&self.state)
    }
}

pub struct ActorNodeBuilder {
    id: ActorID,
    event_processor: Option<EP<ActorStore>>,
    initial_state: Option<ActorStore>,
}

impl Default for ActorNodeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ActorNodeBuilder {
    /// Creates a new builder with the specified ID
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            event_processor: None,
            initial_state: None,
        }
    }

    pub fn with_state(mut self, state: ActorStore) -> Self {
        self.initial_state = Some(state);
        self
    }

    /// Sets the async event processor for the node
    pub fn with_processor<F, Fut>(mut self, processor: F) -> Self
    where
        F: Fn(Event, Arc<Mutex<ActorStore>>, Sender<Event>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Option<Vec<Event>>> + Send + 'static,
    {
        self.event_processor = Some(Box::new(move |event, state, sender| Box::pin(processor(event, state, sender))));
        self
    }

    /// Builds the `ActorNode`, ensuring all required fields are set
    pub fn build(self) -> ActorNode {
        ActorNode {
            id: self.id.to_string(),
            event_processor: self.event_processor.expect("Event processor must be set before building"),
            state: Arc::new(Mutex::new(self.initial_state.expect("Should have initial state"))),
        }
    }
}

type ActorID = String;
type ParentEdges = HashMap<ActorID, Vec<(Sender<Event>, Receiver<Event>)>>;
type ChildEdges = HashMap<ActorID, Vec<Sender<Event>>>;
pub struct ActorDAG {
    parent_edges: ParentEdges,
    child_edges: ChildEdges,
    nodes: HashMap<ActorID, Arc<Mutex<ActorNode>>>,
}

const INCR_KEY: &str = "incr_key";

impl Default for ActorDAG {
    fn default() -> Self {
        Self::new()
    }
}

impl ActorDAG {
    pub fn new() -> Self {
        Self {
            parent_edges: HashMap::new(),
            child_edges: HashMap::new(),
            nodes: HashMap::new(),
        }
    }

    /// Polls each node’s INCR_KEY sum and waits until it stabilizes
    pub async fn wait_until_quiesced(&self) {
        use tokio::time::{sleep, Duration};

        let mut last_sum = 0u64;

        loop {
            // 1) Compute the sum of increments across all nodes
            let mut current_sum = 0u64;
            for node_arc in self.nodes.values() {
                let node = node_arc.lock().await;
                let store_arc = node.get_state();
                let store = store_arc.lock().await;
                // Add whatever is in INCR_KEY for this node (defaults to 0 if absent)
                current_sum += store.get::<u64>(INCR_KEY).copied().unwrap_or(0_u64);
            }

            // 2) Check if it’s unchanged from the last loop
            if current_sum == last_sum {
                // The total hasn't moved; assume the DAG is quiesced
                break;
            }

            // 3) Otherwise, update `last_sum` and wait a bit before re-checking
            last_sum = current_sum;
            sleep(Duration::from_millis(500)).await;
        }
    }

    pub fn read_node(&self, id: ActorID) -> Option<&Arc<Mutex<ActorNode>>> {
        self.nodes.get(&id)
    }

    pub fn set_root(&mut self, node: ActorNode) -> Sender<Event> {
        let (tx, rx) = mpsc::channel(1);
        self.parent_edges.entry(node.id().to_string()).or_default().push((tx.clone(), rx));
        self.add_node(node);
        tx
    }

    pub fn add_node(&mut self, node: ActorNode) {
        self.nodes.insert(node.id().to_string(), Arc::new(Mutex::new(node)));
    }

    pub fn link_parent(&mut self, parent_id: &ActorID, child_id: &ActorID) {
        let (tx, rx) = mpsc::channel(100);
        self.child_edges.entry(parent_id.to_string()).or_default().push(tx.clone());
        self.parent_edges.entry(child_id.to_string()).or_default().push((tx, rx));
    }

    pub async fn spawn_all(&mut self) {
        // Collect all node IDs to avoid borrowing issues while we mutate self
        let node_ids: Vec<_> = self.nodes.keys().cloned().collect();

        // Spawn each node in turn
        for node_id in node_ids {
            self.spawn(&node_id).await;
        }
    }

    pub async fn spawn(&mut self, node_id: &ActorID) {
        let node = self.nodes.get(node_id).unwrap();
        let node_id = { node.lock().await.id() };
        let receivers: Vec<(Sender<Event>, Receiver<Event>)> = self.parent_edges.remove(&node_id).unwrap();
        let senders: Option<Vec<Sender<Event>>> = self.child_edges.remove(&node_id);

        for (requeue, mut receiver) in receivers {
            tokio::spawn({
                let senders = senders.clone();
                let node = node.clone();

                async move {
                    loop {
                        if let Some(event) = receiver.recv().await {
                            let processed_events = {
                                let locked_node = node.lock().await;
                                let event_processor = &locked_node.event_processor;
                                let state = Arc::clone(&locked_node.state);
                                {
                                    let mut locked_state = state.lock().await;
                                    let incr = locked_state.remove::<u64>(INCR_KEY).unwrap_or(0_u64);
                                    locked_state.insert(INCR_KEY, incr + 1);
                                }
                                event_processor(event, state, requeue.clone()).await
                            };

                            if let Some(processed_events) = processed_events {
                                for processed_event in processed_events {
                                    debug!("{:#?}", processed_event);

                                    if let Some(senders) = senders.clone() {
                                        for sender in senders {
                                            if let Err(err) = sender.send(processed_event.clone()).await {
                                                error!("Failed to send event to child: {}. Error: {err}", processed_event.event_type);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }
    }
}

#[async_trait]
pub trait ActorFactory {
    async fn create_actor() -> ActorNode;
}

#[cfg(test)]
mod actor_dag_tests_v2 {
    use super::*;
    use crate::event_sourcing::events::EventType;
    /// A very simple test that creates a DAG with a single node (root),
    /// spawns it, and ensures no errors occur.
    use std::{sync::Arc, time::Duration};
    use tokio::{sync::Mutex, time::sleep};

    #[tokio::test]
    async fn test_single_node_dag() {
        let mut dag = ActorDAG::new();

        // Build a simple root node that does nothing
        let root_node = ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|_event, state, _requeue| {
                Box::pin(async move {
                    state.lock().await.insert("i", 1u64);
                    None
                })
            })
            .build();
        let root_node_id = root_node.id();

        // Set the root node in the DAG, which returns a Sender<Event>
        let root_tx = dag.set_root(root_node);

        // Wrap the ActorDAG in an Arc<Mutex<>> to allow mutable access
        let dag = Arc::new(Mutex::new(dag));

        // Spawn the entire DAG in the background
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // Send one event to the root node
        root_tx
            .send(Event {
                event_type: EventType::GenesisBlock,
                payload: "Hello from test_single_node_dag".to_string(),
            })
            .await
            .unwrap();

        // Give it some time to process
        sleep(Duration::from_millis(50)).await;

        let dag = dag.lock().await;
        dag.wait_until_quiesced().await;
        let node = dag.read_node(root_node_id).unwrap();
        let state = node.lock().await.get_state();

        // Wait a bit to ensure the tasks shut down cleanly
        assert_eq!(state.lock().await.remove::<u64>("i").unwrap(), 1u64);
    }

    #[tokio::test]
    async fn test_mutate_and_inspect_state() {
        // Create the DAG
        let mut dag = ActorDAG::new();

        // Create and configure the root node
        let mut store = ActorStore::new();
        store.insert("count", 0u64);

        let root_node = ActorNodeBuilder::new()
            .with_state(store)
            .with_processor(|_event, state, _requeue| {
                Box::pin(async move {
                    let mut state = state.lock().await;
                    let count: u64 = state.remove("count").unwrap();
                    state.insert("count", count + 1);
                    None
                })
            })
            .build();
        let root_node_id = root_node.id();

        // Set the root node and obtain the sender
        let root_sender = dag.set_root(root_node);

        // Wrap the ActorDAG in an Arc<Mutex<>> to allow mutable access
        let dag = Arc::new(Mutex::new(dag));

        // Spawn the DAG after all setup is complete
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // Send events to the root node
        root_sender
            .send(Event {
                event_type: EventType::GenesisBlock,
                payload: "Event 1".to_string(),
            })
            .await
            .expect("Message should send");
        root_sender
            .send(Event {
                event_type: EventType::GenesisBlock,
                payload: "Event 2".to_string(),
            })
            .await
            .expect("Message should send");

        // Allow processing time
        sleep(Duration::from_millis(100)).await;

        // Verify state has been updated
        let dag = dag.lock().await;
        dag.wait_until_quiesced().await;
        let node = dag.read_node(root_node_id).unwrap();
        let state = node.lock().await.get_state();

        assert_eq!(state.lock().await.get::<u64>("count"), Some(&2));
    }

    #[tokio::test]
    async fn test_event_propagation_between_nodes() {
        // Create the DAG
        let mut dag = ActorDAG::new();

        // Create the root node
        let root_node = ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|event, state, _requeue| {
                Box::pin(async move {
                    debug!("Root processing: {}", event.payload);
                    let mut state = state.lock().await;
                    state.insert("last_event", event.payload.clone());
                    Some(vec![Event {
                        event_type: EventType::NewBlock,
                        payload: "Propagated Event".to_string(),
                    }])
                })
            })
            .build();
        let root_node_id = root_node.id();

        // Create the child node
        let child_node = ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|event, state, _requeue| {
                Box::pin(async move {
                    debug!("Child processing: {}", event.payload);
                    let mut state = state.lock().await;
                    state.insert("received_event", event.payload.clone());
                    None
                })
            })
            .build();
        let child_node_id = child_node.id();

        // Add nodes to the DAG
        let root_sender = dag.set_root(root_node);
        dag.add_node(child_node);

        // Link the root node to the child node
        dag.link_parent(&root_node_id, &child_node_id);

        // Wrap the ActorDAG in an Arc<Mutex<>> to allow mutable access
        let dag = Arc::new(Mutex::new(dag));

        // Spawn the DAG after all setup is complete
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        root_sender
            .send(Event {
                event_type: EventType::GenesisBlock,
                payload: "Root Event".to_string(),
            })
            .await
            .expect("Message should send");

        // Allow processing time
        sleep(Duration::from_millis(100)).await;

        // Verify the child node received the propagated event
        let dag = dag.lock().await;
        dag.wait_until_quiesced().await;
        let node = dag.read_node(child_node_id).unwrap();
        let child_state = node.lock().await.get_state();

        assert_eq!(child_state.lock().await.get::<String>("received_event"), Some(&"Propagated Event".to_string()));
    }

    #[tokio::test]
    async fn test_multi_parent_with_common_ancestor() {
        let mut dag = ActorDAG::new();

        // ------------------------------
        // Common Parent node
        // ------------------------------
        let common_parent_node = ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|_event, _state, _requeue| {
                Box::pin(async {
                    // Always emit a new "NewBlock" event
                    Some(vec![Event {
                        event_type: EventType::NewBlock,
                        payload: "From CommonParent".to_string(),
                    }])
                })
            })
            .build();
        let common_parent_id = common_parent_node.id();

        // ------------------------------
        // Parent1 node
        // ------------------------------
        let parent1_node = ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|event, state, _requeue| {
                Box::pin(async move {
                    // Record the parent's last event
                    let mut locked_state = state.lock().await;
                    locked_state.insert("last_event_parent1", event.payload.clone());
                    // Emit a new event
                    Some(vec![Event {
                        event_type: EventType::NewBlock,
                        payload: "From Parent1".to_string(),
                    }])
                })
            })
            .build();
        let parent1_id = parent1_node.id();

        // ------------------------------
        // Parent2 node
        // ------------------------------
        let parent2_node = ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|event, state, _requeue| {
                Box::pin(async move {
                    // Record the parent's last event
                    let mut locked_state = state.lock().await;
                    locked_state.insert("last_event_parent2", event.payload.clone());
                    // Emit a new event
                    Some(vec![Event {
                        event_type: EventType::NewBlock,
                        payload: "From Parent2".to_string(),
                    }])
                })
            })
            .build();
        let parent2_id = parent2_node.id();

        // ------------------------------
        // Grandchild node
        // ------------------------------
        let grandchild_node = ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|event, state, _requeue| {
                Box::pin(async move {
                    // We'll keep track of *all* payloads we've received
                    let mut locked_state = state.lock().await;
                    let mut all_payloads: Vec<String> = locked_state.get("all_parent_payloads").cloned().unwrap_or_default();
                    all_payloads.push(event.payload.clone());
                    locked_state.insert("all_parent_payloads", all_payloads);

                    None // Does not emit new events
                })
            })
            .build();
        let grandchild_id = grandchild_node.id();

        // ------------------------------
        // DAG Setup
        // ------------------------------
        // Set the CommonParent as root
        let sender = dag.set_root(common_parent_node);

        // Add the other nodes
        dag.add_node(parent1_node);
        dag.add_node(parent2_node);
        dag.add_node(grandchild_node);

        // Link the CommonParent to Parent1 and Parent2
        dag.link_parent(&common_parent_id, &parent1_id);
        dag.link_parent(&common_parent_id, &parent2_id);

        // Link Parent1 and Parent2 to Grandchild
        dag.link_parent(&parent1_id, &grandchild_id);
        dag.link_parent(&parent2_id, &grandchild_id);

        // Wrap the DAG in Arc<Mutex<>> for shared access
        let dag = Arc::new(Mutex::new(dag));

        // Spawn the DAG
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // ------------------------------
        // Send an event to the CommonParent
        // ------------------------------
        sender
            .send(Event {
                event_type: EventType::GenesisBlock,
                payload: "Root Event".to_string(),
            })
            .await
            .expect("Message should send");

        // Allow processing time
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // ------------------------------
        // Verify the Grandchild's state
        // ------------------------------
        let dag_locked = dag.lock().await;
        dag_locked.wait_until_quiesced().await;
        let node = dag_locked.read_node(grandchild_id.clone()).expect("Grandchild node not found");
        let grandchild_actor = node.lock().await;
        let state = grandchild_actor.get_state();
        let locked_state = state.lock().await;

        // We recorded each parent's "From ParentX" payload in "all_parent_payloads"
        let all_parent_payloads: Vec<String> = locked_state.get("all_parent_payloads").cloned().unwrap_or_default();

        // We expect exactly 2 "NewBlock" events from the two parents
        assert_eq!(all_parent_payloads.len(), 2, "Grandchild should receive one NewBlock from each parent");

        // Ensure that the list has "From Parent1" and "From Parent2", in *any* order
        // (If you want to check order, you'd do a direct eq check against e.g. ["From Parent1", "From Parent2"])
        assert!(
            all_parent_payloads.contains(&"From Parent1".to_string()),
            "Should contain the payload from Parent1"
        );
        assert!(
            all_parent_payloads.contains(&"From Parent2".to_string()),
            "Should contain the payload from Parent2"
        );
    }
}
