use async_trait::async_trait;
use convert_case::{Case, Casing};
use rdkafka::{
    admin::{AdminClient, AdminOptions, NewTopic, TopicReplication},
    client::DefaultClientContext,
    error::KafkaResult,
    util::Timeout,
    ClientConfig,
};
use std::time::Duration;
use strum_macros::{AsRefStr, EnumString};

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
        self.as_ref().to_case(Case::Kebab)
    }

    /// Convert a snake case string to an `EventType`
    pub fn from_str(s: &str) -> Result<Self, ()> {
        let pascal_case = s.to_case(Case::Pascal); // Convert back to PascalCase
        pascal_case.parse::<EventType>().map_err(|_| ())
    }
}

#[async_trait]
pub trait Actor {
    /// Perform setup tasks, like creating required topics.
    async fn setup(&self, brokers: &str, topics: &[&str]) {
        // Create the AdminClient
        let client: AdminClient<DefaultClientContext> = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .create()
            .expect("Admin client creation error");

        // Iterate over topics and create each one if it doesn't exist
        for topic in topics {
            match self.topic_exists(&client, topic).await {
                Ok(true) => println!("Topic '{}' already exists.", topic),
                Ok(false) => {
                    if let Err(err) = self.create_topic(&client, topic).await {
                        eprintln!("Failed to create topic '{}': {:?}", topic, err);
                    }
                }
                Err(err) => eprintln!("Failed to check existence of topic '{}': {:?}", topic, err),
            }
        }
    }

    async fn topic_exists(&self, client: &AdminClient<DefaultClientContext>, topic: &str) -> KafkaResult<bool> {
        let metadata = client.inner().fetch_metadata(None, Timeout::Never)?;
        Ok(metadata.topics().iter().any(|t| t.name() == topic))
    }

    /// Create a single Kafka topic.
    async fn create_topic(&self, client: &AdminClient<DefaultClientContext>, topic: &str) -> KafkaResult<()> {
        let new_topic = NewTopic::new(topic, 1, TopicReplication::Fixed(1));
        let res = client
            .create_topics(
                &[new_topic],
                &AdminOptions::new().operation_timeout(Some(Timeout::After(Duration::from_secs(10)))),
            )
            .await?;

        for result in res {
            match result {
                Ok(_) => println!("Topic '{}' created successfully.", topic),
                Err((err, _)) => eprintln!("Failed to create topic '{}': {:?}", topic, err),
            }
        }
        Ok(())
    }

    /// Handles a single event, processing it and potentially publishing a result.
    async fn handle_event(&self, event: Event);

    /// Publishes a processed message to the appropriate topic.
    async fn publish(&self, topic: &str, message: String);
}

#[cfg(test)]
mod eventtype_tests {
    use super::*;

    #[test]
    fn test_eventtype_to_string() {
        let event_type = EventType::PrecomputedBlockPath;
        let snake_case = event_type.to_string();
        assert_eq!(snake_case, "precomputed-block-path");
    }

    #[test]
    fn test_eventtype_from_str_valid() {
        let snake_case = "precomputed-block-path";
        let event_type = EventType::from_str(snake_case).unwrap();
        assert_eq!(event_type, EventType::PrecomputedBlockPath);
    }

    #[test]
    fn test_eventtype_from_str_invalid() {
        let invalid_case = "unknown_event";
        let result = EventType::from_str(invalid_case);
        assert!(result.is_err());
    }
}
