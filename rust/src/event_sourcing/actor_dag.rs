use super::events::{Event, EventType};
use futures::future::BoxFuture;
use log::{error, info};
use std::{any::Any, collections::HashMap, future::Future, sync::Arc};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    watch, Mutex,
};
use tracing::{debug, warn};

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
    id: EventType, // Unique identifier for the node

    // Represents the communication connectors (edges) between this node and its children
    child_edges: HashMap<EventType, Sender<Event>>,

    // Represents the communication connectors (edges) between this node and its parents
    parent_edges: HashMap<EventType, Receiver<Event>>,

    // Represents the actual child nodes connected to this node (the graph nodes)
    child_nodes: Vec<ActorNode>,

    // Internal receiver for incoming events
    receiver: Option<Receiver<Event>>,

    // Sender for outgoing events from this node
    sender: Option<Sender<Event>>,

    // Asynchronous function to process events received by this node
    event_processor: EP<ActorStore>,

    // internal state
    state: Arc<Mutex<ActorStore>>, // State wrapped in Arc<Mutex> for safe sharing

    shutdown_receiver: watch::Receiver<bool>,
}

impl ActorNode {
    pub fn id(&self) -> EventType {
        self.id.clone()
    }

    /// Adds a receiver to the ActorNode and returns the corresponding `mpsc::Receiver`.
    pub fn add_receiver(&mut self, id: EventType) -> mpsc::Receiver<Event> {
        // Create a new channel
        let (sender, receiver) = mpsc::channel(10);

        // Insert the sender into child_edges
        self.child_edges.insert(id, sender);

        // Return the receiver
        receiver
    }

    pub fn get_sender(&mut self) -> Option<mpsc::Sender<Event>> {
        self.sender.clone()
    }

    /// Add a parent connection and return the corresponding sender
    pub fn add_parent(&mut self, id: EventType, actor: &mut ActorNode) -> mpsc::Sender<Event> {
        let (sender, receiver) = mpsc::channel(10);
        self.parent_edges.insert(id.clone(), receiver); // Add to parent edges
        actor.add_edge(id, sender.clone());
        sender // Return the sender for the parent to use
    }

    pub fn add_edge(&mut self, id: EventType, sender: Sender<Event>) {
        self.child_edges.insert(id, sender); // Add edge
    }

    /// Adds a child node to the current node
    pub fn add_child(&mut self, mut child: ActorNode) {
        // Take ownership of the child's sender
        let child_id = child.id.clone();
        if let Some(sender) = child.get_sender() {
            self.add_edge(child_id, sender);
            self.child_nodes.push(child); // Add node
        } else {
            error!("Failed to add child: Sender for {:?} does not exist", child_id);
        }
    }

    /// Gets the size of the graph
    pub fn size(&self) -> usize {
        // Start by counting the current node
        let mut count = 1;

        // Recursively count each child node
        for child_node in &self.child_nodes {
            count += child_node.size();
        }

        count
    }

    /// Recursively spawns tasks for the node and its children, returning their handles
    pub async fn spawn_all(node: Arc<Mutex<Self>>) {
        Box::pin(async move {
            let mut handles = vec![];

            // Spawn a task for the current node
            let node_clone = Arc::clone(&node);
            let handle = tokio::spawn(async move {
                ActorNode::start_processing(node_clone).await;
            });
            handles.push(handle);

            // Recursively spawn tasks for child nodes
            let child_nodes = {
                let mut locked_node = node.lock().await; // Lock the node
                std::mem::take(&mut locked_node.child_nodes) // Take the child nodes for recursion
            };

            for child in child_nodes {
                let child_arc = Arc::new(Mutex::new(child)); // Wrap child in Arc<Mutex>

                ActorNode::spawn_all(child_arc).await; // Recursively spawn child tasks
            }
        })
        .await;
    }

    pub async fn start_processing(node: Arc<Mutex<Self>>) {
        // Move parent_edges out of the node to process them independently
        let parent_edges = {
            let mut locked_node = node.lock().await;
            let mut parent_edges = std::mem::take(&mut locked_node.parent_edges); // Move the parent_edges out
            if let Some(receiver) = locked_node.receiver.take() {
                if parent_edges.insert(locked_node.id.clone(), receiver).is_some() {
                    warn!("Overwrote old receiver in");
                }
            }

            parent_edges
        };

        let shutdown_receiver = {
            // Clone the shutdown_receiver while the lock is held
            let locked_node = node.lock().await;
            locked_node.shutdown_receiver.clone()
        };

        // Spawn tasks for each parent receiver
        for (event_type, mut receiver) in parent_edges {
            let node_clone = Arc::clone(&node);

            tokio::spawn({
                let shutdown_receiver = shutdown_receiver.clone();
                async move {
                    loop {
                        // Check for shutdown signal
                        if *shutdown_receiver.borrow() {
                            info!("Node shutting down for parent receiver {:?}", event_type);
                            break;
                        }

                        // Process incoming events for this receiver
                        if let Some(event) = receiver.recv().await {
                            let processed_events = {
                                let mut locked_node = node_clone.lock().await;
                                let requeue = locked_node.get_sender().unwrap();
                                let event_processor = &locked_node.event_processor;
                                let state = Arc::clone(&locked_node.state);
                                event_processor(event, state, requeue).await
                            };

                            if let Some(processed_events) = processed_events {
                                for processed_event in processed_events {
                                    debug!("{:#?}", processed_event);
                                    let locked_node = node_clone.lock().await;
                                    let child_sender = locked_node.child_edges.get(&processed_event.event_type);

                                    if let Some(sender) = child_sender {
                                        if let Err(err) = sender.send(processed_event.clone()).await {
                                            error!("Failed to send event to child: {}. Error: {err}", processed_event.event_type);
                                        }
                                    } else {
                                        warn!("No child exists to process event for EventType {:?}", processed_event.event_type);
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }

        // Keep the main loop alive until the shutdown signal is received
        while !*shutdown_receiver.borrow() {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        info!("Node processing loop shutting down.");
    }
}

pub struct ActorNodeBuilder {
    id: EventType,
    event_processor: Option<EP<ActorStore>>,
    children: Vec<ActorNode>,
    initial_state: Option<ActorStore>,
}

impl ActorNodeBuilder {
    /// Creates a new builder with the specified ID
    pub fn new(id: EventType) -> Self {
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
    pub fn build(self, shutdown_receiver: watch::Receiver<bool>) -> ActorNode {
        let (tx, rx) = mpsc::channel(10);

        let mut node = ActorNode {
            id: self.id,
            child_edges: HashMap::new(),
            parent_edges: HashMap::new(),
            child_nodes: vec![],
            receiver: Some(rx),
            sender: Some(tx),
            event_processor: self.event_processor.expect("Event processor must be set before building"),
            state: Arc::new(Mutex::new(self.initial_state.expect("Should have initial state"))),
            shutdown_receiver,
        };

        for child in self.children {
            let child_id = child.id.clone();
            let mut child_sender = child.sender.clone();
            node.child_nodes.push(child);
            if let Some(sender) = child_sender.take() {
                node.child_edges.insert(child_id, sender);
            } else {
                error!("Child {:?} does not have a valid sender", child_id);
            }
        }

        node
    }
}

pub trait ActorFactory {
    fn create_actor(shutdown_rx: watch::Receiver<bool>) -> ActorNode;
}

#[cfg(test)]
mod actor_node_tests_v2 {
    use super::*;
    use crate::{constants::POSTGRES_CONNECTION_STRING, event_sourcing::db_logger::DbLogger};
    use log::trace;
    use std::sync::Arc;
    use tokio::sync::{watch, Mutex};
    use tokio_postgres::NoTls;

    #[tokio::test]
    async fn test_size_of_tree() {
        // Shutdown signal
        let (_, shutdown_rx) = watch::channel(false);

        // Create a root node with two children
        let mut root: ActorNode = ActorNodeBuilder::new(EventType::GenesisBlock)
            .with_state(ActorStore::new())
            .with_processor(|_event, _state, _requeue| Box::pin(async { None }))
            .build(shutdown_rx.clone());

        root.add_child(
            ActorNodeBuilder::new(EventType::NewBlock)
                .with_state(ActorStore::new())
                .with_processor(|_event, _state, _requeue| Box::pin(async { None }))
                .build(shutdown_rx.clone()),
        );

        let mut child2 = ActorNodeBuilder::new(EventType::NewBlock)
            .with_state(ActorStore::new())
            .with_processor(|_event, _state, _requeue| Box::pin(async { None }))
            .build(shutdown_rx.clone());

        child2.add_child(
            ActorNodeBuilder::new(EventType::NewBlock)
                .with_state(ActorStore::new())
                .with_processor(|_event, _state, _requeue| Box::pin(async { None }))
                .build(shutdown_rx.clone()),
        );
        root.add_child(child2);

        // Assert that the size matches the number of nodes in the tree (1 root + 2 children)
        assert_eq!(root.size(), 4, "The tree should contain 4 nodes (1 root + 3 children).");
    }

    #[tokio::test]
    async fn test_no_children() {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let mut root: ActorNode = ActorNodeBuilder::new(EventType::GenesisBlock)
            .with_state(ActorStore::new())
            .with_processor(|_event, _state, _requeue| {
                Box::pin(async move {
                    trace!("Root processing event");
                    None // No propagation since there are no children
                })
            })
            .build(shutdown_rx.clone());

        let sender = root.get_sender().expect("Sender should exist");

        let root = Arc::new(Mutex::new(root));
        tokio::spawn({
            let root_clone = Arc::clone(&root);
            async move {
                ActorNode::spawn_all(root_clone).await;
            }
        });

        // Trigger the root node
        sender
            .send(Event {
                event_type: EventType::GenesisBlock,
                payload: "Root event".to_string(),
            })
            .await
            .expect("Message should send");

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Trigger shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    #[tokio::test]
    async fn test_multiple_children() {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let mut root: ActorNode = ActorNodeBuilder::new(EventType::GenesisBlock)
            .with_state(ActorStore::new())
            .with_processor(|event, _state, _requeue| {
                Box::pin(async move {
                    trace!("Root processing: {:?}", event);
                    Some(vec![Event {
                        event_type: EventType::NewBlock,
                        payload: format!("Processed by root: {}", event.payload),
                    }])
                })
            })
            .build(shutdown_rx.clone());

        let child1 = ActorNodeBuilder::new(EventType::NewBlock)
            .with_state(ActorStore::new())
            .with_processor(|event, _state, _requeue| {
                Box::pin(async move {
                    trace!("Child1 processing: {:?}", event);
                    None // Stop propagation
                })
            })
            .build(shutdown_rx.clone());

        let child2 = ActorNodeBuilder::new(EventType::NewBlock)
            .with_state(ActorStore::new())
            .with_processor(|event, _state, _requeue| {
                Box::pin(async move {
                    trace!("Child2 processing: {:?}", event);
                    None // Stop propagation
                })
            })
            .build(shutdown_rx.clone());

        root.add_child(child1);
        root.add_child(child2);

        let sender = root.sender.take().expect("Sender should exist");

        let root = Arc::new(Mutex::new(root));
        tokio::spawn({
            let root_clone = Arc::clone(&root);
            async move {
                ActorNode::spawn_all(root_clone).await;
            }
        });

        // Trigger the root node
        sender
            .send(Event {
                event_type: EventType::GenesisBlock,
                payload: "Root event".to_string(),
            })
            .await
            .expect("Message should send");

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Trigger shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    #[tokio::test]
    async fn test_event_processor_filtering() {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let mut root: ActorNode = ActorNodeBuilder::new(EventType::GenesisBlock)
            .with_state(ActorStore::new())
            .with_processor(|_event, _state, _requeue| {
                Box::pin(async move {
                    trace!("Root processing event");
                    None // Do not propagate
                })
            })
            .build(shutdown_rx.clone());

        root.add_child(
            ActorNodeBuilder::new(EventType::NewBlock)
                .with_state(ActorStore::new())
                .with_processor(|_event, _state, _requeue| {
                    Box::pin(async move {
                        trace!("Child processing event");
                        None
                    })
                })
                .build(shutdown_rx.clone()),
        );

        let sender = root.sender.take().expect("Sender should exist");

        let root = Arc::new(Mutex::new(root));
        tokio::spawn({
            let root_clone = Arc::clone(&root);
            async move {
                ActorNode::spawn_all(root_clone).await;
            }
        });

        // Trigger the root node
        sender
            .send(Event {
                event_type: EventType::GenesisBlock,
                payload: "Root event".to_string(),
            })
            .await
            .expect("Message should send");

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Trigger shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    #[tokio::test]
    async fn test_mutate_and_inspect_state() {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let mut store = ActorStore::new();
        store.insert("count", 0u64);
        let mut root: ActorNode = ActorNodeBuilder::new(EventType::GenesisBlock)
            .with_state(store)
            .with_processor(|event, state, _requeue| {
                Box::pin(async move {
                    let mut locked_state = state.lock().await; // Lock the state
                    let current_count: u64 = locked_state.remove("count").unwrap();
                    locked_state.insert("count", current_count + 1u64); // Increment the counter
                    println!("Processing event: {:?}, updated state: {:?}", event, current_count);
                    Some(vec![Event {
                        event_type: EventType::NewBlock,
                        payload: format!("Processed: {}", event.payload),
                    }])
                })
            })
            .build(shutdown_rx.clone());

        let sender = root.get_sender().expect("Sender should exist");

        let root = Arc::new(Mutex::new(root));
        tokio::spawn({
            let root_clone = Arc::clone(&root);
            async move {
                ActorNode::spawn_all(root_clone).await;
            }
        });

        // Send events to the root
        sender
            .send(Event {
                event_type: EventType::GenesisBlock,
                payload: "Root event 1".to_string(),
            })
            .await
            .expect("Message should send");

        sender
            .send(Event {
                event_type: EventType::GenesisBlock,
                payload: "Root event 2".to_string(),
            })
            .await
            .expect("Message should send");

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Trigger shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");

        //Verify state has been updated
        let root_clone = Arc::clone(&root);
        let locked_root = root_clone.lock().await;
        let state = locked_root.state.lock().await;
        assert_eq!(
            state.get("count").unwrap() as &u64,
            &2_u64,
            "State counter should have incremented by the number of processed events."
        );
    }

    #[tokio::test]
    async fn test_child_node_with_db_connection() {
        // Database setup
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Failed to connect to the database");

        // Spawn the connection handler
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        // Setup the DbLogger
        let logger = DbLogger::builder(client)
            .name("child_node_log")
            .add_column("height BIGINT")
            .add_column("payload TEXT")
            .distinct_columns(&["payload"])
            .build(&None)
            .await
            .expect("Failed to build child_node_log and view");

        // ActorNode with database connection in its state
        let mut store = ActorStore::new();
        store.insert("logger", logger);
        let mut root = ActorNodeBuilder::new(EventType::NewBlock)
            .with_state(store) // Pass the DbLogger as state
            .with_processor(|event, state: Arc<Mutex<ActorStore>>, _requeue| {
                Box::pin(async move {
                    let mut state = state.lock().await;

                    let logger: DbLogger = state.remove("logger").unwrap();

                    // Insert the event into the database
                    logger
                        .insert(
                            &[&0_i64, &event.payload],
                            0, // Height is not relevant in this test
                        )
                        .await
                        .expect("Failed to insert event into database");

                    state.insert("logger", logger);
                    None // No propagation
                })
            })
            .build(watch::channel(false).1); // Use a dummy shutdown_receiver

        // Send events to the child node
        let sender = root.get_sender().expect("Sender should exist");

        let root = Arc::new(Mutex::new(root));
        tokio::spawn({
            let root_clone = Arc::clone(&root);
            async move {
                ActorNode::spawn_all(root_clone).await;
            }
        });

        // Trigger the child node with events
        sender
            .send(Event {
                event_type: EventType::NewBlock,
                payload: "Payload 1".to_string(),
            })
            .await
            .expect("Message should send");

        sender
            .send(Event {
                event_type: EventType::NewBlock,
                payload: "Payload 2".to_string(),
            })
            .await
            .expect("Message should send");

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        {
            let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
                .await
                .expect("Failed to connect to the database");

            // Spawn the connection handler
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("Connection error: {}", e);
                }
            });

            // Verify that the events were inserted into the database
            let query = "SELECT * FROM child_node_log";
            let rows = client.query(query, &[]).await.expect("Failed to query database");

            assert_eq!(rows.len(), 2, "Expected 2 rows in the database");

            let payloads: Vec<String> = rows.iter().map(|row| row.get("payload")).collect();
            assert!(payloads.contains(&"Payload 1".to_string()));
            assert!(payloads.contains(&"Payload 2".to_string()));
        }
    }

    #[tokio::test]
    async fn test_event_routing_to_specific_receiver() {
        use std::sync::Arc;
        use tokio::sync::watch;

        // Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Create the root actor
        let mut state = ActorStore::new();
        state.insert("i", 1u64);
        let mut root: ActorNode = ActorNodeBuilder::new(EventType::GenesisBlock)
            .with_state(state)
            .with_processor(|_event, state, _requeue| {
                Box::pin(async move {
                    let mut state = state.lock().await;
                    let i: u64 = state.remove("i").unwrap();
                    if i % 2 == 0 {
                        state.insert("i", i + 1u64);
                        Some(vec![Event {
                            event_type: EventType::NewBlock,
                            payload: String::from("Payload for NewBlock"),
                        }])
                    } else {
                        state.insert("i", i + 1u64);
                        Some(vec![Event {
                            event_type: EventType::PrecomputedBlockPath,
                            payload: String::from("Payload for PrecomputedBlockPath"),
                        }])
                    }
                })
            }) // Root does not process events
            .build(shutdown_rx.clone());

        // Add two receivers for specific EventTypes
        let mut receiver1 = root.add_receiver(EventType::NewBlock);
        let mut receiver2 = root.add_receiver(EventType::PrecomputedBlockPath);

        // Wrap the root in an Arc<Mutex> for shared ownership
        let root = Arc::new(Mutex::new(root));

        // Spawn the root actor
        tokio::spawn({
            let root_clone = Arc::clone(&root);
            async move {
                ActorNode::spawn_all(root_clone).await;
            }
        });

        // Consume the sender once
        let sender = root.lock().await.get_sender().unwrap();

        // Scenario 1: Send an event for NewBlock
        sender
            .send(Event {
                event_type: EventType::NewBlock,
                payload: "Payload for NewBlock".to_string(),
            })
            .await
            .expect("Failed to send event for NewBlock");

        sender
            .send(Event {
                event_type: EventType::PrecomputedBlockPath,
                payload: "Payload for PrecomputedBlockPath".to_string(),
            })
            .await
            .expect("Failed to send event for PrecomputedBlockPath");

        // Verify that only the first receiver gets the event
        if let Ok(Some(received_event)) = tokio::time::timeout(tokio::time::Duration::from_secs(2), receiver1.recv()).await {
            assert_eq!(received_event.event_type, EventType::NewBlock);
            assert_eq!(received_event.payload, "Payload for NewBlock");
        } else {
            panic!("Receiver 1 did not receive the expected event within the timeout.");
        }

        // Ensure the second receiver does not receive the event
        assert!(receiver1.try_recv().is_err(), "Receiver 2 should not receive the event.");

        if let Ok(Some(received_event)) = tokio::time::timeout(tokio::time::Duration::from_secs(2), receiver2.recv()).await {
            assert_eq!(received_event.event_type, EventType::PrecomputedBlockPath);
            assert_eq!(received_event.payload, "Payload for PrecomputedBlockPath");
        } else {
            panic!("Receiver 2 did not receive the expected event within the timeout.");
        }

        // Ensure the first receiver does not receive the event
        assert!(receiver2.try_recv().is_err(), "Receiver 1 should not receive the event.");

        // Signal shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_processing_with_add_parent_api() {
        use std::sync::Arc;
        use tokio::sync::{watch, Mutex};

        // Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Create the actor
        let mut state = ActorStore::new();
        state.insert("i", 0u64);
        let mut child_actor = ActorNodeBuilder::new(EventType::NewBlock)
            .with_state(state)
            .with_processor(|event, state, _requeue| {
                Box::pin(async move {
                    let mut state = state.lock().await;
                    let i: u64 = state.remove("i").unwrap();
                    state.insert("i", i + 1u64);
                    println!("Processing event: {:?}", event);
                    None
                })
            })
            .build(shutdown_rx.clone());

        // Create two parent actors
        let mut parent1 = ActorNodeBuilder::new(EventType::GenesisBlock)
            .with_state(ActorStore::new())
            .with_processor(|event, _state, _requeue| {
                Box::pin(async move {
                    println!("Parent 1 processing: {:?}", event);
                    Some(vec![event])
                })
            })
            .build(shutdown_rx.clone());

        let mut parent2 = ActorNodeBuilder::new(EventType::PrecomputedBlockPath)
            .with_state(ActorStore::new())
            .with_processor(|event, _state, _requeue| {
                Box::pin(async move {
                    println!("Parent 2 processing: {:?}", event);
                    Some(vec![event])
                })
            })
            .build(shutdown_rx.clone());

        // Add two parents for one child
        child_actor.add_parent(EventType::GenesisBlock, &mut parent1);
        child_actor.add_parent(EventType::PrecomputedBlockPath, &mut parent2);

        // Wrap the child actor in Arc<Mutex> for shared ownership
        let child_actor = Arc::new(Mutex::new(child_actor));
        let parent1 = Arc::new(Mutex::new(parent1));
        let parent2 = Arc::new(Mutex::new(parent2));

        // Spawn the child actor
        tokio::spawn({
            let p = Arc::clone(&parent1);

            async move {
                ActorNode::spawn_all(p).await;
            }
        });
        tokio::spawn({
            let p = Arc::clone(&parent2);

            async move {
                ActorNode::spawn_all(p).await;
            }
        });
        tokio::spawn({
            let ch = Arc::clone(&child_actor);

            async move {
                ActorNode::start_processing(ch).await;
            }
        });

        // Send events to the parent connections
        parent1
            .lock()
            .await
            .get_sender()
            .unwrap()
            .send(Event {
                event_type: EventType::GenesisBlock,
                payload: "Event from Parent 1".to_string(),
            })
            .await
            .expect("Failed to send event from Parent 1");

        parent2
            .lock()
            .await
            .get_sender()
            .unwrap()
            .send(Event {
                event_type: EventType::PrecomputedBlockPath,
                payload: "Event from Parent 2".to_string(),
            })
            .await
            .expect("Failed to send event from Parent 2");

        // Allow time for processing
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Verify the child actor's state
        let locked_child = child_actor.lock().await;
        let locked_state = locked_child.state.lock().await;
        println!("{:#?}", locked_state.get("i").unwrap() as &u64);
        assert_eq!(locked_state.get("i").unwrap() as &u64, &2_u64, "State should have incremented twice");

        // Signal shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }
}