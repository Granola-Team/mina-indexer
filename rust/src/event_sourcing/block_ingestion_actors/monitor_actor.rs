use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{event_sourcing::payloads::ActorHeightPayload, utility::Throttler};
use async_trait::async_trait;
use futures::lock::Mutex;
use log::{debug, info};
use std::{
    collections::{HashMap, VecDeque},
    sync::{atomic::AtomicUsize, Arc},
};

pub struct MonitorActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub actor_heights: Arc<Mutex<HashMap<String, u64>>>,
    pub height_spreads: Arc<Mutex<VecDeque<u64>>>, // FIFO queue for height spreads
    pub sliding_window_size: usize,
    pub throttler: Mutex<Throttler>,
}

impl MonitorActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>, sliding_window_size: usize, throttle_interval: Option<usize>) -> Self {
        Self {
            id: "MonitorActor".to_string(),
            shared_publisher,
            actor_heights: Arc::new(Mutex::new(HashMap::new())),
            height_spreads: Arc::new(Mutex::new(VecDeque::with_capacity(sliding_window_size))),
            sliding_window_size,
            throttler: Mutex::new(Throttler::new(throttle_interval.unwrap_or(1))), // Default to 1 if no throttle
        }
    }

    async fn get_running_average(&self) -> f64 {
        let spreads = self.height_spreads.lock().await;
        spreads.iter().copied().map(|x| x as f64).sum::<f64>() / spreads.len() as f64
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
        let running_avg = self.get_running_average().await;
        for (actor, height) in actor_heights.iter() {
            debug!("{}: Actor {} has processed up to height {}", self.id(), actor, height);
        }
        info!("{}: On average, actors are within {:.2} heights of each other", self.id(), running_avg);
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

            // Update the FIFO queue
            let mut spreads = self.height_spreads.lock().await;
            if spreads.len() == self.sliding_window_size {
                spreads.pop_back(); // Remove the oldest value
            }
            spreads.push_front(height_spread);

            if spreads.len() < self.sliding_window_size {
                return;
            }
            drop(spreads);

            let running_avg = self.get_running_average().await;

            // Check throttle condition
            let mut throttler = self.throttler.lock().await;
            if throttler.should_invoke() {
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
        let actor = MonitorActor::new(Arc::clone(&shared_publisher), 3, None); // Set min_data_points to 3

        // Subscribe to the shared publisher to capture output events
        let mut receiver = shared_publisher.subscribe_high_priority();

        // Send ActorHeight events
        let payloads = vec![
            ActorHeightPayload {
                actor: "Actor1".to_string(),
                height: 10,
            },
            ActorHeightPayload {
                actor: "Actor2".to_string(),
                height: 20,
            },
            ActorHeightPayload {
                actor: "Actor3".to_string(),
                height: 15,
            },
            ActorHeightPayload {
                actor: "Actor4".to_string(),
                height: 25,
            },
        ];

        for payload in &payloads {
            actor
                .handle_event(Event {
                    event_type: EventType::ActorHeight,
                    payload: sonic_rs::to_string(payload).unwrap(),
                })
                .await;
        }

        // Verify the FIFO queue and running average
        let spreads = actor.height_spreads.lock().await;
        assert_eq!(spreads.len(), 3); // Ensure only the last 3 spreads are kept
        drop(spreads);

        let running_avg = actor.get_running_average().await;
        let expected_average = (10.0 + 10.0 + 15.0) / 3.0; // Last three spreads are [10, 10, 15]
        assert!((running_avg - expected_average).abs() < 0.01);

        // Verify that the RunningAvgHeightSpread event is published
        if let Ok(event) = tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            assert_eq!(event.unwrap().event_type, EventType::RunningAvgHeightSpread);
        } else {
            panic!("Expected RunningAvgHeightSpread event was not published.");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_monitor_actor_respects_sliding_window() -> anyhow::Result<()> {
        use crate::event_sourcing::payloads::ActorHeightPayload;

        // Create a shared publisher and the MonitorActor with min_data_points = 3
        let shared_publisher = Arc::new(SharedPublisher::new(200));
        let actor = MonitorActor::new(Arc::clone(&shared_publisher), 3, None);

        // Subscribe to the shared publisher to capture output events
        let mut receiver = shared_publisher.subscribe_high_priority();

        // Send fewer events than min_data_points
        let payloads = vec![
            ActorHeightPayload {
                actor: "Actor1".to_string(),
                height: 10,
            },
            ActorHeightPayload {
                actor: "Actor2".to_string(),
                height: 20,
            },
        ];

        for payload in &payloads {
            actor
                .handle_event(Event {
                    event_type: EventType::ActorHeight,
                    payload: sonic_rs::to_string(payload).unwrap(),
                })
                .await;
        }

        // Verify that no events are published since the threshold is not met
        assert!(
            receiver.try_recv().is_err(),
            "No events should be published as min_data_points threshold is not met"
        );

        // Send one more event to meet the threshold
        actor
            .handle_event(Event {
                event_type: EventType::ActorHeight,
                payload: sonic_rs::to_string(&ActorHeightPayload {
                    actor: "Actor3".to_string(),
                    height: 15,
                })
                .unwrap(),
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
        let actor = MonitorActor::new(Arc::clone(&shared_publisher), 3, Some(2)); // Sliding window size = 3, throttle = 2

        // Subscribe to the shared publisher to capture output events
        let mut receiver = shared_publisher.subscribe_high_priority();

        // Send events to trigger the actor
        let payloads = vec![
            ActorHeightPayload {
                actor: "Actor1".to_string(),
                height: 10,
            },
            ActorHeightPayload {
                actor: "Actor2".to_string(),
                height: 20,
            },
            ActorHeightPayload {
                actor: "Actor3".to_string(),
                height: 15,
            },
            ActorHeightPayload {
                actor: "Actor4".to_string(),
                height: 25,
            },
            ActorHeightPayload {
                actor: "Actor5".to_string(),
                height: 30,
            },
            ActorHeightPayload {
                actor: "Actor6".to_string(),
                height: 35,
            },
        ];

        // Send all events to the actor
        for payload in &payloads {
            actor
                .handle_event(Event {
                    event_type: EventType::ActorHeight,
                    payload: sonic_rs::to_string(payload).unwrap(),
                })
                .await;
        }

        // Verify that only throttled events are published
        let mut published_events = 0;
        while let Ok(event) = tokio::time::timeout(std::time::Duration::from_millis(500), receiver.recv()).await {
            let event = event.unwrap();
            assert_eq!(event.event_type, EventType::RunningAvgHeightSpread);
            published_events += 1;
        }

        // Validate throttling after the sliding window is full
        assert_eq!(published_events, 2, "Only 2 events should be published due to throttling");

        Ok(())
    }
}
