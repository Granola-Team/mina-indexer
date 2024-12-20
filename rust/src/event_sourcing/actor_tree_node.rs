use futures::future::BoxFuture;
use log::error;
use std::{collections::HashMap, future::Future, sync::Arc};
use tokio::sync::{mpsc, Mutex};

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
    id: EventType,                                                                   // Unique identifier for the node
    children: HashMap<EventType, mpsc::Sender<Event>>,                               // Channels to children identified by EventType
    receiver: mpsc::Receiver<Event>,                                                 // Internal receiver for events
    sender: Option<mpsc::Sender<Event>>,                                             // Internal sender for this node
    event_processor: Box<dyn Fn(Event) -> BoxFuture<'static, Option<Event>> + Send>, // Async event processor
}

impl ActorNode {
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

    /// Trigger an event for this node
    pub async fn trigger(&self, event: Event) {
        if let Some(sender) = &self.sender {
            if let Err(err) = sender.send(event).await {
                error!("Failed to trigger event: {:?}", err);
            }
        } else {
            error!("Node {:?} does not have a valid sender to trigger events", self.id);
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
            receiver: rx,
            sender: Some(tx),
            event_processor: self.event_processor.expect("Event processor must be set before building"),
        };

        for mut child in self.children {
            if let Some(sender) = child.sender.take() {
                node.children.insert(child.id.clone(), sender);
            } else {
                error!("Child {:?} does not have a valid sender", child.id);
            }
        }

        node
    }
}

#[cfg(test)]
mod actor_node_tests {
    use super::*;
    use log::trace;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_no_children() {
        let safe_root = Arc::new(Mutex::new(
            ActorNodeBuilder::new(EventType::GenesisBlock)
                .with_processor(|_| {
                    Box::pin(async move {
                        trace!("Root processing event");
                        None // No propagation since there are no children
                    })
                })
                .build(),
        ));

        let processing_root = Arc::clone(&safe_root);
        tokio::spawn(async move {
            ActorNode::start_processing(processing_root).await;
        });

        safe_root
            .lock()
            .await
            .trigger(Event {
                event_type: EventType::GenesisBlock,
                payload: "Root event".to_string(),
            })
            .await;

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    #[tokio::test]
    async fn test_multiple_children() {
        let safe_root = Arc::new(Mutex::new(
            ActorNodeBuilder::new(EventType::GenesisBlock)
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
                .build(),
        ));

        let processing_root = Arc::clone(&safe_root);
        tokio::spawn(async move {
            ActorNode::start_processing(processing_root).await;
        });

        safe_root
            .lock()
            .await
            .trigger(Event {
                event_type: EventType::GenesisBlock,
                payload: "Root event".to_string(),
            })
            .await;

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    #[tokio::test]
    async fn test_event_processor_filtering() {
        let safe_root = Arc::new(Mutex::new(
            ActorNodeBuilder::new(EventType::GenesisBlock)
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
                .build(),
        ));

        let processing_root = Arc::clone(&safe_root);
        tokio::spawn(async move {
            ActorNode::start_processing(processing_root).await;
        });

        safe_root
            .lock()
            .await
            .trigger(Event {
                event_type: EventType::GenesisBlock,
                payload: "Root event".to_string(),
            })
            .await;

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
}
