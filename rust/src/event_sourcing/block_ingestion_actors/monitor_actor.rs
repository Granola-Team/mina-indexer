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
    pub running_average_spread: Arc<Mutex<f64>>,
    pub spread_count: AtomicUsize,
    pub min_data_points: usize,
    pub throttle: Option<usize>,
}

impl MonitorActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>, min_data_points: usize, throttle: Option<usize>) -> Self {
        Self {
            id: "MonitorActor".to_string(),
            shared_publisher,
            actor_heights: Arc::new(Mutex::new(HashMap::new())),
            running_average_spread: Arc::new(Mutex::new(0.0)),
            spread_count: AtomicUsize::new(0),
            min_data_points,
            throttle,
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
        let running_avg = *self.running_average_spread.lock().await;
        for (actor, height) in actor_heights.iter() {
            println!("{}: Actor {} has processed up to height {}", self.id(), actor, height);
        }
        println!("{}: On average, actors are within {:.2} heights of each other", self.id(), running_avg);
    }

    async fn handle_event(&self, event: Event) {
        if event.event_type == EventType::ActorHeight {
            let payload: ActorHeightPayload = sonic_rs::from_str(&event.payload).unwrap();
            let mut actor_heights = self.actor_heights.lock().await;
            actor_heights.insert(payload.actor, payload.height);

            // Calculate the current spread
            let (min, max) = actor_heights
                .values()
                .fold((u64::MAX, u64::MIN), |(min, max), &value| (min.min(value), max.max(value)));
            let height_spread = max - min;

            // Update the running average spread
            let mut running_avg = self.running_average_spread.lock().await;
            let count = self.spread_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            if count < self.min_data_points {
                return;
            }
            *running_avg = ((*running_avg * (count - 1) as f64) + height_spread as f64) / count as f64;

            if self.throttle.map_or(true, |throttle| count % throttle == 0) {
                self.publish(Event {
                    event_type: EventType::RunningAvgHeightSpread,
                    payload: running_avg.to_string(),
                });
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
        shared_publisher::SharedPublisher,
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn test_monitor_actor_calculates_running_average() -> anyhow::Result<()> {
        use crate::event_sourcing::payloads::ActorHeightPayload;

        // Create a shared publisher and the MonitorActor
        let shared_publisher = Arc::new(SharedPublisher::new(200));
        let actor = MonitorActor::new(Arc::clone(&shared_publisher), 0, None);

        // Subscribe to the shared publisher to capture output events
        let mut receiver = shared_publisher.subscribe_high_priority();

        // Send ActorHeight events
        let payload1 = ActorHeightPayload {
            actor: "Actor1".to_string(),
            height: 10,
        };
        let payload2 = ActorHeightPayload {
            actor: "Actor2".to_string(),
            height: 20,
        };
        let payload3 = ActorHeightPayload {
            actor: "Actor3".to_string(),
            height: 15,
        };

        actor
            .handle_event(Event {
                event_type: EventType::ActorHeight,
                payload: sonic_rs::to_string(&payload1).unwrap(),
            })
            .await;

        actor
            .handle_event(Event {
                event_type: EventType::ActorHeight,
                payload: sonic_rs::to_string(&payload2).unwrap(),
            })
            .await;

        actor
            .handle_event(Event {
                event_type: EventType::ActorHeight,
                payload: sonic_rs::to_string(&payload3).unwrap(),
            })
            .await;

        // Verify the running average of the spread is correctly published
        let expected_averages = vec![
            0.0,  // First event: Spread = 0, Average = 0
            5.0,  // Second event: Spread = 10, Average = (0 + 10) / 2 = 5
            6.67, // Third event: Spread = 10, Average = (0 + 10 + 10) / 3 = 6.67
        ];

        for expected_avg in expected_averages {
            if let Ok(evt) = tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
                let event = evt.unwrap();
                assert_eq!(event.event_type, EventType::RunningAvgHeightSpread);

                let payload: f64 = event.payload.parse().unwrap();
                println!("{payload}");
                assert!((payload - expected_avg).abs() < 0.01); // Allow minor floating-point deviations
            } else {
                panic!("Expected HeightSpread event was not published.");
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_monitor_actor_respects_min_data_points() -> anyhow::Result<()> {
        use crate::event_sourcing::payloads::ActorHeightPayload;

        // Create a shared publisher and the MonitorActor with min_data_points = 3
        let shared_publisher = Arc::new(SharedPublisher::new(200));
        let actor = MonitorActor::new(Arc::clone(&shared_publisher), 3, None);

        // Subscribe to the shared publisher to capture output events
        let mut receiver = shared_publisher.subscribe_high_priority();

        // Send fewer events than min_data_points
        let payload1 = ActorHeightPayload {
            actor: "Actor1".to_string(),
            height: 10,
        };
        let payload2 = ActorHeightPayload {
            actor: "Actor2".to_string(),
            height: 20,
        };

        actor
            .handle_event(Event {
                event_type: EventType::ActorHeight,
                payload: sonic_rs::to_string(&payload1).unwrap(),
            })
            .await;

        actor
            .handle_event(Event {
                event_type: EventType::ActorHeight,
                payload: sonic_rs::to_string(&payload2).unwrap(),
            })
            .await;

        // Verify that no events are published since the threshold is not met
        assert!(
            receiver.try_recv().is_err(),
            "No events should be published as min_data_points threshold is not met"
        );

        // Send one more event to meet the threshold
        let payload3 = ActorHeightPayload {
            actor: "Actor3".to_string(),
            height: 15,
        };

        actor
            .handle_event(Event {
                event_type: EventType::ActorHeight,
                payload: sonic_rs::to_string(&payload3).unwrap(),
            })
            .await;

        // Verify that an event is now published
        if let Ok(event) = tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            assert_eq!(event.unwrap().event_type, EventType::RunningAvgHeightSpread);
        } else {
            panic!("Expected RunningAvgHeightSpread event was not published.");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_monitor_actor_respects_throttle() -> anyhow::Result<()> {
        use crate::event_sourcing::payloads::ActorHeightPayload;

        // Create a shared publisher and the MonitorActor with throttle = 2
        let shared_publisher = Arc::new(SharedPublisher::new(200));
        let actor = MonitorActor::new(Arc::clone(&shared_publisher), 0, Some(2));

        // Subscribe to the shared publisher to capture output events
        let mut receiver = shared_publisher.subscribe_high_priority();

        // Send events to trigger the actor
        let payload1 = ActorHeightPayload {
            actor: "Actor1".to_string(),
            height: 10,
        };
        let payload2 = ActorHeightPayload {
            actor: "Actor2".to_string(),
            height: 20,
        };

        actor
            .handle_event(Event {
                event_type: EventType::ActorHeight,
                payload: sonic_rs::to_string(&payload1).unwrap(),
            })
            .await;

        // Verify that no event is published after the first two events (throttling)
        assert!(
            receiver.try_recv().is_err(),
            "No events should be published until the throttle condition is met"
        );

        actor
            .handle_event(Event {
                event_type: EventType::ActorHeight,
                payload: sonic_rs::to_string(&payload2).unwrap(),
            })
            .await;

        // Verify that an event is published after the third event
        if let Ok(event) = tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            assert_eq!(event.unwrap().event_type, EventType::RunningAvgHeightSpread);
        } else {
            panic!("Expected RunningAvgHeightSpread event was not published after throttle condition was met.");
        }

        Ok(())
    }
}
