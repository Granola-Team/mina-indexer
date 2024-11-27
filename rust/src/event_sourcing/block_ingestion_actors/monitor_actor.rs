use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::event_sourcing::payloads::ActorHeightPayload;
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
    pub modulo: AtomicUsize,
    pub update_throttle: usize,
}

impl MonitorActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>, update_throttle: usize) -> Self {
        Self {
            id: "MonitorActor".to_string(),
            shared_publisher,
            actor_heights: Arc::new(Mutex::new(HashMap::new())),
            modulo: AtomicUsize::new(1),
            update_throttle,
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

            if self.modulo.load(std::sync::atomic::Ordering::SeqCst) % self.update_throttle == 0 {
                self.modulo.store(1, std::sync::atomic::Ordering::SeqCst);
                self.publish(Event {
                    event_type: EventType::HeightSpread,
                    payload: height_spread.to_string(),
                });
            } else {
                self.modulo.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish_high_priority(event);
    }
}
#[cfg(test)]
mod monitor_actor_tests {
    use super::*;
    use crate::event_sourcing::{
        events::{Event, EventType},
        payloads::ActorHeightPayload,
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn test_monitor_actor_height_spread() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let monitor_actor = MonitorActor::new(Arc::clone(&shared_publisher), 10);

        let mut receiver = shared_publisher.subscribe_high_priority();

        // First ActorHeight event (height = 10)
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

        // Second ActorHeight event (height = 25)
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

        // Third ActorHeight event (height = 35)
        let actor_height_3 = ActorHeightPayload {
            actor: "actor3".to_string(),
            height: 35,
        };
        monitor_actor
            .handle_event(Event {
                event_type: EventType::ActorHeight,
                payload: sonic_rs::to_string(&actor_height_3).unwrap(),
            })
            .await;

        // Fourth ActorHeight event (height = 40)
        let actor_height_4 = ActorHeightPayload {
            actor: "actor4".to_string(),
            height: 40,
        };
        monitor_actor
            .handle_event(Event {
                event_type: EventType::ActorHeight,
                payload: sonic_rs::to_string(&actor_height_4).unwrap(),
            })
            .await;

        // Expect no event yet because we're still below the 10th event
        let no_event = tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(no_event.is_err(), "No event should be published yet");

        // Fifth ActorHeight event (height = 50)
        let actor_height_5 = ActorHeightPayload {
            actor: "actor5".to_string(),
            height: 50,
        };
        monitor_actor
            .handle_event(Event {
                event_type: EventType::ActorHeight,
                payload: sonic_rs::to_string(&actor_height_5).unwrap(),
            })
            .await;

        // Sixth ActorHeight event (height = 60)
        let actor_height_6 = ActorHeightPayload {
            actor: "actor6".to_string(),
            height: 60,
        };
        monitor_actor
            .handle_event(Event {
                event_type: EventType::ActorHeight,
                payload: sonic_rs::to_string(&actor_height_6).unwrap(),
            })
            .await;

        // Seventh ActorHeight event (height = 70)
        let actor_height_7 = ActorHeightPayload {
            actor: "actor7".to_string(),
            height: 70,
        };
        monitor_actor
            .handle_event(Event {
                event_type: EventType::ActorHeight,
                payload: sonic_rs::to_string(&actor_height_7).unwrap(),
            })
            .await;

        // Eighth ActorHeight event (height = 80)
        let actor_height_8 = ActorHeightPayload {
            actor: "actor8".to_string(),
            height: 80,
        };
        monitor_actor
            .handle_event(Event {
                event_type: EventType::ActorHeight,
                payload: sonic_rs::to_string(&actor_height_8).unwrap(),
            })
            .await;

        // Ninth ActorHeight event (height = 90)
        let actor_height_9 = ActorHeightPayload {
            actor: "actor9".to_string(),
            height: 90,
        };
        monitor_actor
            .handle_event(Event {
                event_type: EventType::ActorHeight,
                payload: sonic_rs::to_string(&actor_height_9).unwrap(),
            })
            .await;

        // Tenth ActorHeight event (height = 100), should trigger the publication
        let actor_height_10 = ActorHeightPayload {
            actor: "actor10".to_string(),
            height: 100,
        };
        monitor_actor
            .handle_event(Event {
                event_type: EventType::ActorHeight,
                payload: sonic_rs::to_string(&actor_height_10).unwrap(),
            })
            .await;

        // Expect the HeightSpread event to be published
        let event = tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv())
            .await
            .expect("Expected a published event")
            .expect("Event received");

        assert_eq!(event.event_type, EventType::HeightSpread);
        assert_eq!(event.payload, "90".to_string()); // Expecting the spread based on 100th event
    }
}
