use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::stream::payloads::ActorHeightPayload;
use async_trait::async_trait;
use futures::lock::Mutex;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc},
};

pub struct MonitorActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub actor_heights: Arc<Mutex<HashMap<String, u64>>>,
}

impl MonitorActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "MonitorActor".to_string(),
            shared_publisher,
            actor_heights: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Actor for MonitorActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        static DUMMY: AtomicUsize = AtomicUsize::new(0);
        &DUMMY
    }

    async fn report(&self) {
        let actor_heights = self.actor_heights.lock().await;
        for (actor, height) in actor_heights.iter() {
            println!("{}: Actor {} has processed up to height {}", self.id(), actor, height);
        }
    }

    async fn handle_event(&self, event: Event) {
        if event.event_type == EventType::ActorHeight {
            let payload: ActorHeightPayload = sonic_rs::from_str(&event.payload).unwrap();
            let mut actor_heights = self.actor_heights.lock().await;
            actor_heights.insert(payload.actor, payload.height);

            let (min, max) = actor_heights
                .values()
                .fold((u64::MAX, u64::MIN), |(min, max), &value| (min.min(value), max.max(value)));

            // Compute the absolute difference
            let height_spread = max - min;

            self.publish(Event {
                event_type: EventType::HeightSpread,
                payload: height_spread.to_string(),
            });
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[cfg(test)]
mod monitor_actor_tests {
    use super::*;
    use crate::stream::{
        events::{Event, EventType},
        payloads::ActorHeightPayload,
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn test_monitor_actor_height_spread() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let monitor_actor = MonitorActor::new(Arc::clone(&shared_publisher));

        let mut receiver = shared_publisher.subscribe();

        // Add first ActorHeight event
        let actor_height_1 = ActorHeightPayload {
            actor: "actor1".to_string(),
            height: 10,
        };
        monitor_actor
            .handle_event(Event {
                event_type: EventType::ActorHeight,
                payload: sonic_rs::to_string(&actor_height_1).unwrap(),
            })
            .await;

        // Add second ActorHeight event
        let actor_height_2 = ActorHeightPayload {
            actor: "actor2".to_string(),
            height: 25,
        };
        monitor_actor
            .handle_event(Event {
                event_type: EventType::ActorHeight,
                payload: sonic_rs::to_string(&actor_height_2).unwrap(),
            })
            .await;

        {
            // Expect the event to be published with the height spread (10 - 10 = 0)
            let event = tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv())
                .await
                .expect("Expected a published event")
                .expect("Event received");

            assert_eq!(event.event_type, EventType::HeightSpread);
            assert_eq!(event.payload, "0".to_string());
        }

        {
            // Expect the event to be published with the height spread (25 - 10 = 15)
            let event = tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv())
                .await
                .expect("Expected a published event")
                .expect("Event received");

            assert_eq!(event.event_type, EventType::HeightSpread);
            assert_eq!(event.payload, "15".to_string());
        }
    }
}
