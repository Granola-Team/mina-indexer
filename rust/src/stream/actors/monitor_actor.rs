use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    constants::{FILE_PUBLISHER_ACTOR_ID, TRANSITION_FRONTIER_DISTANCE},
    stream::payloads::ActorHeightPayload,
};
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
        let file_publisher_height = actor_heights.get(FILE_PUBLISHER_ACTOR_ID).unwrap_or(&0u64);
        for (actor, height) in actor_heights.iter() {
            if height < &(file_publisher_height - TRANSITION_FRONTIER_DISTANCE as u64) {
                eprintln!(
                    "{}: Actor {} fallen too far behind in processing and will lose data. height {}",
                    self.id(),
                    actor,
                    height
                );
            } else {
                println!("{}: Actor {} has processed up to height {}", self.id(), actor, height);
            }
        }
    }

    async fn handle_event(&self, event: Event) {
        if event.event_type == EventType::ActorHeight {
            let payload: ActorHeightPayload = sonic_rs::from_str(&event.payload).unwrap();
            let mut actor_heights = self.actor_heights.lock().await;
            actor_heights.insert(payload.actor, payload.height);
        }
    }

    fn publish(&self, event: Event) {
        self.shared_publisher.publish(event);
    }
}