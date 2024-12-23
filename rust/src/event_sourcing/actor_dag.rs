use super::events::{Event, EventType};
use futures::future::BoxFuture;
use log::{error, info};
use std::{collections::HashMap, future::Future, sync::Arc};
use tokio::sync::{mpsc, watch, Mutex};

pub struct Stateless;

type EP<S> = Box<dyn Fn(Event, Arc<Mutex<S>>) -> BoxFuture<'static, Option<Event>> + Send + Sync>;

pub struct ActorNode<S> {
    id: EventType, // Unique identifier for the node

    // Represents the communication connectors (edges) between this node and its children
    child_edges: HashMap<EventType, mpsc::Sender<Event>>,

    // Represents the actual child nodes connected to this node (the graph nodes)
    child_nodes: Vec<ActorNode<S>>,

    // Internal receiver for incoming events
    receiver: mpsc::Receiver<Event>,

    // Sender for outgoing events from this node
    sender: Option<mpsc::Sender<Event>>,

    // Asynchronous function to process events received by this node
    event_processor: EP<S>,

    // internal state
    state: Arc<Mutex<S>>, // State wrapped in Arc<Mutex> for safe sharing

    shutdown_receiver: watch::Receiver<bool>,
}

impl<S> ActorNode<S>
where
    S: Send + 'static,
{
    /// Adds a child node to the current node
    pub fn add_child(&mut self, mut child: ActorNode<S>) {
        // Take ownership of the child's sender
        let child_id = child.id.clone();
        if let Some(sender) = child.sender.take() {
            self.child_edges.insert(child_id.clone(), sender); // Add edge
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

    /// Starts processing messages asynchronously
    pub async fn start_processing(node: Arc<Mutex<Self>>) {
        loop {
            let is_shutdown = {
                let locked_node = node.lock().await; // Lock the node to access the shutdown_receiver
                let shutdown_receiver = locked_node.shutdown_receiver.clone(); // Clone the receiver
                drop(locked_node); // Explicitly drop the lock
                let shutdown_sigal = *shutdown_receiver.borrow(); // Access the shutdown signal value
                shutdown_sigal
            };

            if is_shutdown {
                info!("Node shutting down");
                break; // Exit the loop on shutdown
            }

            // Process an incoming event with a timeout
            let event = {
                let mut locked_node = node.lock().await;
                tokio::time::timeout(tokio::time::Duration::from_millis(1), locked_node.receiver.recv())
                    .await
                    .ok()
                    .flatten()
            };

            if let Some(event) = event {
                let processed_event = {
                    let locked_node = node.lock().await;
                    let event_processor = &locked_node.event_processor;
                    let state = Arc::clone(&locked_node.state);
                    event_processor(event, state).await
                };

                if let Some(processed_event) = processed_event {
                    let children = {
                        let locked_node = node.lock().await;
                        locked_node.child_edges.clone()
                    };

                    for (_, sender) in children {
                        if let Err(err) = sender.send(processed_event.clone()).await {
                            error!("Failed to send event to child: {:?}", err);
                        }
                    }
                }
            }

            // Add a short sleep to avoid busy-waiting
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        }
    }
}

pub struct ActorNodeBuilder<S> {
    id: EventType,
    event_processor: Option<EP<S>>,
    children: Vec<ActorNode<S>>,
    initial_state: Option<S>,
}

impl<S> ActorNodeBuilder<S> {
    /// Creates a new builder with the specified ID
    pub fn new(id: EventType) -> Self {
        Self {
            id,
            event_processor: None,
            children: Vec::new(),
            initial_state: None,
        }
    }

    pub fn with_state(mut self, state: S) -> Self {
        self.initial_state = Some(state);
        self
    }

    /// Sets the async event processor for the node
    pub fn with_processor<F, Fut>(mut self, processor: F) -> Self
    where
        F: Fn(Event, Arc<Mutex<S>>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Option<Event>> + Send + 'static,
    {
        self.event_processor = Some(Box::new(move |event, state| Box::pin(processor(event, state))));
        self
    }

    /// Builds the `ActorNode`, ensuring all required fields are set
    pub fn build(self, shutdown_receiver: watch::Receiver<bool>) -> ActorNode<S> {
        let (tx, rx) = mpsc::channel(10);

        let mut node = ActorNode {
            id: self.id,
            child_edges: HashMap::new(),
            child_nodes: vec![],
            receiver: rx,
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
    type State;

    fn create_actor(shutdown_rx: watch::Receiver<bool>) -> Arc<Mutex<ActorNode<Self::State>>>;
}

#[cfg(test)]
mod actor_node_tests {
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
        let mut root: ActorNode<Stateless> = ActorNodeBuilder::new(EventType::GenesisBlock)
            .with_state(Stateless {})
            .with_processor(|_event, _state| Box::pin(async { None }))
            .build(shutdown_rx.clone());

        root.add_child(
            ActorNodeBuilder::new(EventType::NewBlock)
                .with_state(Stateless {})
                .with_processor(|_event, _state| Box::pin(async { None }))
                .build(shutdown_rx.clone()),
        );

        let mut child2 = ActorNodeBuilder::new(EventType::NewBlock)
            .with_state(Stateless {})
            .with_processor(|_event, _state| Box::pin(async { None }))
            .build(shutdown_rx.clone());

        child2.add_child(
            ActorNodeBuilder::new(EventType::NewBlock)
                .with_state(Stateless {})
                .with_processor(|_event, _state| Box::pin(async { None }))
                .build(shutdown_rx.clone()),
        );
        root.add_child(child2);

        // Assert that the size matches the number of nodes in the tree (1 root + 2 children)
        assert_eq!(root.size(), 4, "The tree should contain 4 nodes (1 root + 3 children).");
    }

    #[tokio::test]
    async fn test_no_children() {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let mut root: ActorNode<Stateless> = ActorNodeBuilder::new(EventType::GenesisBlock)
            .with_state(Stateless {})
            .with_processor(|_event, _state| {
                Box::pin(async move {
                    trace!("Root processing event");
                    None // No propagation since there are no children
                })
            })
            .build(shutdown_rx.clone());

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
    async fn test_multiple_children() {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let mut root: ActorNode<Stateless> = ActorNodeBuilder::new(EventType::GenesisBlock)
            .with_state(Stateless {})
            .with_processor(|event, _state| {
                Box::pin(async move {
                    trace!("Root processing: {:?}", event);
                    Some(Event {
                        event_type: EventType::NewBlock,
                        payload: format!("Processed by root: {}", event.payload),
                    })
                })
            })
            .build(shutdown_rx.clone());

        let child1 = ActorNodeBuilder::new(EventType::NewBlock)
            .with_state(Stateless {})
            .with_processor(|event, _state| {
                Box::pin(async move {
                    trace!("Child1 processing: {:?}", event);
                    None // Stop propagation
                })
            })
            .build(shutdown_rx.clone());

        let child2 = ActorNodeBuilder::new(EventType::NewBlock)
            .with_state(Stateless {})
            .with_processor(|event, _state| {
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

        let mut root: ActorNode<Stateless> = ActorNodeBuilder::new(EventType::GenesisBlock)
            .with_state(Stateless {})
            .with_processor(|_event, _state| {
                Box::pin(async move {
                    trace!("Root processing event");
                    None // Do not propagate
                })
            })
            .build(shutdown_rx.clone());

        root.add_child(
            ActorNodeBuilder::new(EventType::NewBlock)
                .with_state(Stateless {})
                .with_processor(|_event, _state| {
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

    #[derive(Debug, Default)]
    pub struct CounterState {
        count: u32,
    }

    #[tokio::test]
    async fn test_mutate_and_inspect_state() {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let mut root: ActorNode<CounterState> = ActorNodeBuilder::new(EventType::GenesisBlock)
            .with_state(CounterState { count: 0 })
            .with_processor(|event, state| {
                Box::pin(async move {
                    let mut locked_state = state.lock().await; // Lock the state
                    locked_state.count += 1; // Increment the counter
                    trace!("Processing event: {:?}, updated state: {:?}", event, locked_state.count);
                    Some(Event {
                        event_type: EventType::NewBlock,
                        payload: format!("Processed: {}", event.payload),
                    })
                })
            })
            .build(shutdown_rx.clone());

        let sender = root.sender.take().expect("Sender should exist");

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
        assert_eq!(state.count, 2, "State counter should have incremented by the number of processed events.");
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

        struct State {
            logger: DbLogger,
        }

        // ActorNode with database connection in its state
        let mut root = ActorNodeBuilder::new(EventType::NewBlock)
            .with_state(State { logger }) // Pass the DbLogger as state
            .with_processor(|event, state: Arc<Mutex<State>>| {
                Box::pin(async move {
                    let state = state.lock().await;

                    // Insert the event into the database
                    state
                        .logger
                        .insert(
                            &[&0_i64, &event.payload],
                            0, // Height is not relevant in this test
                        )
                        .await
                        .expect("Failed to insert event into database");

                    None // No propagation
                })
            })
            .build(watch::channel(false).1); // Use a dummy shutdown_receiver

        // Send events to the child node
        let sender = root.sender.take().expect("Sender should exist");

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
}
