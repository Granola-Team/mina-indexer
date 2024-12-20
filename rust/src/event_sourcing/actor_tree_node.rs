use futures::future::BoxFuture;
use log::error;
use std::{collections::HashMap, future::Future, sync::Arc};
use tokio::{
    sync::{mpsc, Mutex},
    task::JoinHandle,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventType {
    GenesisBlock,
    NewBlock,
}

#[derive(Debug, Clone)]
pub struct Event {
    pub event_type: EventType,
    pub payload: String,
}

pub struct ActorNode {
    id: EventType, // Unique identifier for the node
    children: HashMap<EventType, mpsc::Sender<Event>>,
    child_nodes: Vec<ActorNode>,                                                     // Channels to children identified by EventType
    receiver: mpsc::Receiver<Event>,                                                 // Internal receiver for events
    sender: Option<mpsc::Sender<Event>>,                                             // Internal sender for this node
    event_processor: Box<dyn Fn(Event) -> BoxFuture<'static, Option<Event>> + Send>, // Async event processor
}

impl ActorNode {
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
    pub fn spawn(mut self) -> BoxFuture<'static, Vec<JoinHandle<()>>> {
        Box::pin(async move {
            let mut handles = vec![];

            let child_nodes = self.child_nodes.drain(..).collect::<Vec<ActorNode>>();

            // Spawn a task for the current node
            let handle = tokio::spawn(async move {
                let node = Arc::new(Mutex::new(self));
                ActorNode::start_processing(node).await;
            });
            handles.push(handle);

            // Recursively spawn tasks for child nodes
            for child in child_nodes {
                // Move each child into the recursive call
                let mut child_handles = child.spawn().await; // Await recursive call
                handles.append(&mut child_handles); // Merge handles
            }

            handles
        })
    }

    /// Starts processing messages asynchronously
    pub async fn start_processing(node: Arc<Mutex<Self>>) {
        loop {
            let event = {
                let mut locked_node = node.lock().await; // Acquire the lock
                locked_node.receiver.recv().await // Receive an event
            };

            if let Some(event) = event {
                let processed_event = {
                    let locked_node = node.lock().await; // Acquire the lock again
                    (locked_node.event_processor)(event).await // Await the async processor
                };

                if let Some(processed_event) = processed_event {
                    let children = {
                        let locked_node = node.lock().await; // Acquire the lock again
                        locked_node.children.clone() // Clone the children map
                    };

                    // Send the processed event to all children
                    for (_, sender) in children {
                        if let Err(err) = sender.send(processed_event.clone()).await {
                            error!("Failed to send event to child: {:?}", err);
                        }
                    }
                }
            }
        }
    }
}

pub struct ActorNodeBuilder {
    id: EventType,
    event_processor: Option<Box<dyn Fn(Event) -> BoxFuture<'static, Option<Event>> + Send>>,
    children: Vec<ActorNode>,
}

impl ActorNodeBuilder {
    /// Creates a new builder with the specified ID
    pub fn new(id: EventType) -> Self {
        Self {
            id,
            event_processor: None,
            children: Vec::new(),
        }
    }

    /// Sets the async event processor for the node
    pub fn with_processor<F, Fut>(mut self, processor: F) -> Self
    where
        F: Fn(Event) -> Fut + Send + 'static,
        Fut: Future<Output = Option<Event>> + Send + 'static,
    {
        self.event_processor = Some(Box::new(move |event| Box::pin(processor(event))));
        self
    }

    /// Adds a child to the node
    pub fn add_child(mut self, child: ActorNode) -> Self {
        self.children.push(child);
        self
    }

    /// Builds the `ActorNode`, ensuring all required fields are set
    pub fn build(self) -> ActorNode {
        let (tx, rx) = mpsc::channel(10);

        let mut node = ActorNode {
            id: self.id,
            children: HashMap::new(),
            child_nodes: vec![], // Store child nodes directly
            receiver: rx,
            sender: Some(tx),
            event_processor: self.event_processor.expect("Event processor must be set before building"),
        };

        for child in self.children {
            let child_id = child.id.clone();
            let mut child_sender = child.sender.clone();
            node.child_nodes.push(child);
            if let Some(sender) = child_sender.take() {
                node.children.insert(child_id, sender);
            } else {
                error!("Child {:?} does not have a valid sender", child_id);
            }
        }

        node
    }
}
#[cfg(test)]
mod actor_node_tests {
    use super::*;
    use log::trace;

    #[tokio::test]
    async fn test_size_of_tree() {
        // Create a root node with two children
        let root = ActorNodeBuilder::new(EventType::GenesisBlock)
            .with_processor(|_event| {
                Box::pin(async { None }) // No propagation for this test
            })
            .add_child(
                ActorNodeBuilder::new(EventType::NewBlock)
                    .with_processor(|_event| {
                        Box::pin(async { None }) // No propagation for this test
                    })
                    .build(),
            )
            .add_child(
                ActorNodeBuilder::new(EventType::NewBlock)
                    .with_processor(|_event| {
                        Box::pin(async { None }) // No propagation for this test
                    })
                    .add_child(
                        ActorNodeBuilder::new(EventType::NewBlock)
                            .with_processor(|_event| {
                                Box::pin(async { None }) // No propagation for this test
                            })
                            .build(),
                    )
                    .build(),
            )
            .build();

        // Assert that the size matches the number of nodes in the tree (1 root + 2 children)
        assert_eq!(root.size(), 4, "The tree should contain 4 nodes (1 root + 3 children).");
    }

    #[tokio::test]
    async fn test_no_children() {
        let mut root = ActorNodeBuilder::new(EventType::GenesisBlock)
            .with_processor(|_| {
                Box::pin(async move {
                    trace!("Root processing event");
                    None // No propagation since there are no children
                })
            })
            .build();

        let sender = root.sender.take().expect("Sender should exist");

        tokio::spawn(async { root.spawn().await });

        // Trigger the root node
        sender
            .send(Event {
                event_type: EventType::GenesisBlock,
                payload: "Root event".to_string(),
            })
            .await
            .expect("Message should send");

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    #[tokio::test]
    async fn test_multiple_children() {
        let mut root = ActorNodeBuilder::new(EventType::GenesisBlock)
            .with_processor(|event| {
                Box::pin(async move {
                    trace!("Root processing: {:?}", event);
                    Some(Event {
                        event_type: EventType::NewBlock,
                        payload: format!("Processed by root: {}", event.payload),
                    })
                })
            })
            .add_child(
                ActorNodeBuilder::new(EventType::NewBlock)
                    .with_processor(|event| {
                        Box::pin(async move {
                            trace!("Child1 processing: {:?}", event);
                            None // Stop propagation
                        })
                    })
                    .build(),
            )
            .add_child(
                ActorNodeBuilder::new(EventType::NewBlock)
                    .with_processor(|event| {
                        Box::pin(async move {
                            trace!("Child2 processing: {:?}", event);
                            None // Stop propagation
                        })
                    })
                    .build(),
            )
            .build();

        let sender = root.sender.take().expect("Sender should exist");

        // Spawn all tasks
        tokio::spawn(async { root.spawn().await });

        // Trigger the root node
        sender
            .send(Event {
                event_type: EventType::GenesisBlock,
                payload: "Root event".to_string(),
            })
            .await
            .expect("Message should send");

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    #[tokio::test]
    async fn test_event_processor_filtering() {
        let mut root = ActorNodeBuilder::new(EventType::GenesisBlock)
            .with_processor(|_| {
                Box::pin(async move {
                    trace!("Root processing event");
                    None // Do not propagate
                })
            })
            .add_child(
                ActorNodeBuilder::new(EventType::NewBlock)
                    .with_processor(|_| {
                        Box::pin(async move {
                            trace!("Child processing event");
                            None
                        })
                    })
                    .build(),
            )
            .build();

        let sender = root.sender.take().expect("Sender should exist");

        tokio::spawn(async { root.spawn().await });

        // Trigger the root node
        sender
            .send(Event {
                event_type: EventType::GenesisBlock,
                payload: "Root event".to_string(),
            })
            .await
            .expect("Message should send");

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
}
