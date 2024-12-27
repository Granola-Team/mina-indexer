use super::events::Event;
use futures::future::BoxFuture;
use log::error;
use std::{any::Any, collections::HashMap, future::Future, sync::Arc};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    watch, Mutex,
};
use tracing::debug;

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
    children: Vec<ActorNode>,
    initial_state: Option<ActorStore>,
}

impl ActorNodeBuilder {
    /// Creates a new builder with the specified ID
    pub fn new(id: ActorID) -> Self {
        Self {
            id,
            event_processor: None,
            children: Vec::new(),
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

impl ActorDAG {
    pub fn new() -> Self {
        Self {
            parent_edges: HashMap::new(),
            child_edges: HashMap::new(),
            nodes: HashMap::new(),
        }
    }

    pub fn read_node(&self, id: ActorID) -> Option<&Arc<Mutex<ActorNode>>> {
        self.nodes.get(&id)
    }

    pub fn set_root(&mut self, node: ActorNode) -> Sender<Event> {
        let (tx, rx) = mpsc::channel(1);
        self.parent_edges.entry(node.id().to_string()).or_insert_with(Vec::new).push((tx.clone(), rx));
        self.add_node(node);
        tx
    }

    pub fn add_node(&mut self, node: ActorNode) {
        self.nodes.insert(node.id().to_string(), Arc::new(Mutex::new(node)));
    }

    pub fn link_parent(&mut self, parent_id: &ActorID, child_id: &ActorID) {
        let (tx, rx) = mpsc::channel(1);
        self.child_edges.entry(parent_id.to_string()).or_insert_with(Vec::new).push(tx.clone());
        self.parent_edges.entry(child_id.to_string()).or_insert_with(Vec::new).push((tx, rx));
    }

    pub async fn spawn_all(&mut self, shutdown_receiver: watch::Receiver<bool>) {
        // Collect all node IDs to avoid borrowing issues while we mutate self
        let node_ids: Vec<_> = self.nodes.keys().cloned().collect();

        // Spawn each node in turn
        for node_id in node_ids {
            self.spawn(&node_id, shutdown_receiver.clone()).await;
        }
    }

    pub async fn spawn(&mut self, node_id: &ActorID, shutdown_receiver: watch::Receiver<bool>) {
        let node = self.nodes.get(node_id).unwrap();
        let node_id = { node.lock().await.id() };
        let receivers: Vec<(Sender<Event>, Receiver<Event>)> = self.parent_edges.remove(&node_id).unwrap();
        let senders: Option<Vec<Sender<Event>>> = self.child_edges.remove(&node_id);

        for (requeue, mut receiver) in receivers {
            tokio::spawn({
                let senders = senders.clone();
                let node = node.clone();
                let shutdown_receiver = shutdown_receiver.clone();

                async move {
                    loop {
                        // check for shutdown signal
                        if *shutdown_receiver.borrow() {
                            break;
                        }

                        if let Some(event) = receiver.recv().await {
                            let processed_events = {
                                let locked_node = node.lock().await;
                                let event_processor = &locked_node.event_processor;
                                let state = Arc::clone(&locked_node.state);
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

pub trait ActorFactory {
    fn create_actor(shutdown_rx: watch::Receiver<bool>) -> ActorNode;
}

#[cfg(test)]
mod actor_dag_tests_v2 {
    use super::*;
    use crate::event_sourcing::events::EventType;
    /// A very simple test that creates a DAG with a single node (root),
    /// spawns it, and ensures no errors occur.
    use std::sync::Arc;
    use tokio::{
        sync::{watch, Mutex},
        time::{sleep, Duration},
    };

    #[tokio::test]
    async fn test_single_node_dag() {
        // Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let mut dag = ActorDAG::new();

        // Build a simple root node that does nothing
        let root_node_id = "RootNodeActor".to_string();
        let root_node = ActorNodeBuilder::new(root_node_id.to_string())
            .with_state(ActorStore::new())
            .with_processor(|_event, state, _requeue| {
                Box::pin(async move {
                    state.lock().await.insert("i", 1u64);
                    None
                })
            })
            .build();

        // Set the root node in the DAG, which returns a Sender<Event>
        let root_tx = dag.set_root(root_node);

        // Wrap the ActorDAG in an Arc<Mutex<>> to allow mutable access
        let dag = Arc::new(Mutex::new(dag));

        // Spawn the entire DAG in the background
        tokio::spawn({
            let shutdown_rx = shutdown_rx.clone();
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all(shutdown_rx).await;
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

        // Trigger shutdown
        shutdown_tx.send(true).unwrap();

        let dag = dag.lock().await;
        let node = dag.read_node(root_node_id).unwrap();
        let state = node.lock().await.get_state();

        // Wait a bit to ensure the tasks shut down cleanly
        assert_eq!(state.lock().await.remove::<u64>("i").unwrap(), 1u64);
    }
}
