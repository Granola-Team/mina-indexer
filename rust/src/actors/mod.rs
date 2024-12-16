use async_trait::async_trait;
use convert_case::{Case, Casing};
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    message::Message,
};
use strum_macros::{AsRefStr, EnumString};
use tokio_stream::StreamExt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, AsRefStr, EnumString)]
pub enum EventType {
    PrecomputedBlockPath,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub event_type: EventType,
    pub payload: String,
}

impl EventType {
    /// Convert the `EventType` to snake case string
    pub fn to_string(&self) -> String {
        self.as_ref().to_case(Case::Snake)
    }

    /// Convert a snake case string to an `EventType`
    pub fn from_str(s: &str) -> Result<Self, ()> {
        let pascal_case = s.to_case(Case::Pascal); // Convert back to PascalCase
        pascal_case.parse::<EventType>().map_err(|_| ())
    }
}

#[async_trait]
pub trait Actor {
    /// Consumes messages from assigned topics and delegates to `handle_event`.
    async fn consume(&self, consumer: &StreamConsumer, topics: &[&str]) {
        // Subscribe to the topics
        consumer.subscribe(topics).expect("Failed to subscribe to topics");

        // Start streaming messages
        let mut stream = consumer.stream();
        while let Some(result) = stream.next().await {
            match result {
                Ok(message) => {
                    if let Some(Ok(payload)) = message.payload_view::<str>() {
                        let topic = message.topic();
                        let event = Event {
                            event_type: EventType::from_str(topic).unwrap(),
                            payload: payload.to_string(),
                        };
                        self.handle_event(event).await;
                    }
                }
                Err(err) => eprintln!("Error consuming message: {:?}", err),
            }
        }
    }

    /// Handles a single event, processing it and potentially publishing a result.
    async fn handle_event(&self, event: Event);

    /// Publishes a processed message to the appropriate topic.
    async fn publish(&self, topic: &str, message: String);
}
