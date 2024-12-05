use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    constants::{MEMO_PREFIX, MINA_NAME_SERVICE_ADDRESS},
    event_sourcing::payloads::{CanonicalUserCommandLogPayload, UsernamePayload},
};
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct UsernameActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub entries_processed: AtomicUsize,
}

impl UsernameActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "UsernameActor".to_string(),
            shared_publisher,
            entries_processed: AtomicUsize::new(0),
        }
    }

    async fn process_user_command(&self, payload: &CanonicalUserCommandLogPayload) {
        if payload.receiver == MINA_NAME_SERVICE_ADDRESS && payload.memo.starts_with(MEMO_PREFIX) {
            if let Some(username) = payload.memo.strip_prefix(MEMO_PREFIX) {
                let username_payload = UsernamePayload {
                    username: username.to_string(),
                    address: payload.sender.to_string(),
                    height: payload.height,
                    state_hash: payload.state_hash.to_string(),
                    canonical: payload.canonical,
                };

                // Publish the username event
                let event = Event {
                    event_type: EventType::Username,
                    payload: sonic_rs::to_string(&username_payload).unwrap(),
                };

                self.publish(event);
            }
        }
    }
}

#[async_trait]
impl Actor for UsernameActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.entries_processed
    }

    async fn handle_event(&self, event: Event) {
        if let EventType::CanonicalUserCommandLog = event.event_type {
            let payload: CanonicalUserCommandLogPayload = sonic_rs::from_str(&event.payload).unwrap();
            // user commands that aren't canonical, and weren't previously canonical
            // don't need to be published
            if !payload.canonical && !payload.was_canonical {
                return;
            }
            self.process_user_command(&payload).await;
        }
    }

    fn publish(&self, event: Event) {
        self.entries_processed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.shared_publisher.publish(event);
    }
}

#[cfg(test)]
mod username_actor_tests {
    use super::*;
    use crate::event_sourcing::{
        events::{Event, EventType},
        payloads::{CanonicalUserCommandLogPayload, UsernamePayload},
    };
    use std::sync::Arc;
    use tokio::time::timeout;

    async fn setup_actor() -> (Arc<UsernameActor>, tokio::sync::broadcast::Receiver<Event>) {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = Arc::new(UsernameActor::new(Arc::clone(&shared_publisher)));
        let receiver = shared_publisher.subscribe();
        (actor, receiver)
    }

    #[tokio::test]
    async fn test_publish_username_event_with_valid_recipient() {
        let (actor, mut receiver) = setup_actor().await;

        // Prepare a valid CanonicalUserCommandLogPayload
        let payload = CanonicalUserCommandLogPayload {
            receiver: MINA_NAME_SERVICE_ADDRESS.to_string(),
            memo: "Name: testuser".to_string(),
            sender: "B62qtestsender".to_string(),
            height: 100,
            state_hash: "testhash".to_string(),
            canonical: true,
            was_canonical: false,
            ..Default::default()
        };

        let event = Event {
            event_type: EventType::CanonicalUserCommandLog,
            payload: sonic_rs::to_string(&payload).unwrap(),
        };

        // Handle the event
        actor.handle_event(event).await;

        // Expect a Username event to be published
        if let Ok(received_event) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            let published_event = received_event.unwrap();
            assert_eq!(published_event.event_type, EventType::Username);

            // Deserialize the payload and verify the fields
            let username_payload: UsernamePayload = sonic_rs::from_str(&published_event.payload).unwrap();

            assert_eq!(username_payload.username, "testuser");
            assert_eq!(username_payload.address, "B62qtestsender");
            assert_eq!(username_payload.height, 100);
            assert_eq!(username_payload.state_hash, "testhash");
            assert!(username_payload.canonical);
        } else {
            panic!("Expected Username event not received");
        }
    }

    #[tokio::test]
    async fn test_ignore_event_with_invalid_recipient() {
        let (actor, mut receiver) = setup_actor().await;

        // Prepare a payload with an invalid recipient
        let payload = CanonicalUserCommandLogPayload {
            receiver: "B62qInvalidRecipient".to_string(),
            memo: "Name: testuser".to_string(),
            sender: "B62qtestsender".to_string(),
            height: 101,
            state_hash: "testhash".to_string(),
            canonical: true,
            was_canonical: false,
            ..Default::default()
        };

        let event = Event {
            event_type: EventType::CanonicalUserCommandLog,
            payload: sonic_rs::to_string(&payload).unwrap(),
        };

        // Handle the event
        actor.handle_event(event).await;

        // Expect no events to be published
        let result = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(result.is_err(), "Event with invalid recipient should not publish an event");
    }

    #[tokio::test]
    async fn test_ignore_event_with_invalid_memo_prefix() {
        let (actor, mut receiver) = setup_actor().await;

        // Prepare a payload with an invalid memo prefix
        let payload = CanonicalUserCommandLogPayload {
            receiver: MINA_NAME_SERVICE_ADDRESS.to_string(),
            memo: "InvalidPrefix:testuser".to_string(), // Incorrect prefix
            sender: "B62qtestsender".to_string(),
            height: 102,
            state_hash: "testhash".to_string(),
            canonical: true,
            was_canonical: false,
            ..Default::default()
        };

        let event = Event {
            event_type: EventType::CanonicalUserCommandLog,
            payload: sonic_rs::to_string(&payload).unwrap(),
        };

        // Handle the event
        actor.handle_event(event).await;

        // Expect no events to be published
        let result = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(result.is_err(), "Event with invalid memo prefix should not publish an event");
    }

    #[tokio::test]
    async fn test_ignore_non_canonical_user_command() {
        let (actor, mut receiver) = setup_actor().await;

        // Prepare a payload that is non-canonical
        let payload = CanonicalUserCommandLogPayload {
            receiver: MINA_NAME_SERVICE_ADDRESS.to_string(),
            memo: "Name: ignoreduser".to_string(),
            sender: "B62qignoredsender".to_string(),
            height: 101,
            state_hash: "ignoredhash".to_string(),
            canonical: false,
            was_canonical: false,
            ..Default::default()
        };

        let event = Event {
            event_type: EventType::CanonicalUserCommandLog,
            payload: sonic_rs::to_string(&payload).unwrap(),
        };

        // Handle the event
        actor.handle_event(event).await;

        // Expect no events to be published
        let result = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(result.is_err(), "Non-canonical payload should not publish an event");
    }
}
