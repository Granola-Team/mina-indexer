use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::stream::{models::Height, payloads::*};
use anyhow::Result;
use async_trait::async_trait;
use futures::lock::Mutex;
use std::{
    collections::{HashMap, VecDeque},
    sync::{atomic::AtomicUsize, Arc},
};

pub struct InternalCommandCanonicityActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
    pub block_canonicity_queue: Arc<Mutex<VecDeque<BlockCanonicityUpdatePayload>>>,
    pub internal_commands: Arc<Mutex<HashMap<Height, Vec<InternalCommandPayload>>>>,
}

impl InternalCommandCanonicityActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "InternalCommandCanonicityActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
            block_canonicity_queue: Arc::new(Mutex::new(VecDeque::new())),
            internal_commands: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn process_internal_commands(&self) -> Result<(), &'static str> {
        let mut queue = self.block_canonicity_queue.lock().await;

        while let Some(update) = queue.pop_front() {
            let commands = self.internal_commands.lock().await;
            if let Some(entries) = commands.get(&Height(update.height)) {
                for entry in entries {
                    let payload = InternalCommandCanonicityPayload {
                        internal_command_type: entry.internal_command_type.clone(),
                        height: entry.height,
                        state_hash: entry.state_hash.to_string(),
                        timestamp: entry.timestamp,
                        amount_nanomina: entry.amount_nanomina,
                        recipient: entry.recipient.to_string(),
                        source: entry.source.clone(),
                        canonical: update.canonical,
                        was_canonical: update.was_canonical,
                    };
                    self.publish(Event {
                        event_type: EventType::InternalCommandCanonicityUpdate,
                        payload: sonic_rs::to_string(&payload).unwrap(),
                    });
                }
            } else {
                queue.push_back(update);
                drop(queue);
                break;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Actor for InternalCommandCanonicityActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.events_published
    }

    async fn handle_event(&self, event: Event) {
        match event.event_type {
            EventType::BlockCanonicityUpdate => {
                let mut queue = self.block_canonicity_queue.lock().await;
                queue.push_back(sonic_rs::from_str(&event.payload).unwrap());
                drop(queue);
                self.process_internal_commands()
                    .await
                    .expect("Expected to publish internal command canonicity updates");
            }
            EventType::InternalCommand => {
                let event_payload: InternalCommandPayload = sonic_rs::from_str(&event.payload).unwrap();
                let mut internal_commands = self.internal_commands.lock().await;
                internal_commands
                    .entry(Height(event_payload.height))
                    .or_insert_with(Vec::new)
                    .push(event_payload);
                drop(internal_commands);
                self.process_internal_commands()
                    .await
                    .expect("Expected to publish internal command canonicity updates");
            }
            EventType::TransitionFrontier => {
                let height: u64 = sonic_rs::from_str(&event.payload).unwrap();
                let mut internal_commands = self.internal_commands.lock().await;
                internal_commands.retain(|key, _| key.0 > height);
                drop(internal_commands);
            }
            _ => return,
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[cfg(test)]
mod internal_command_canonicity_actor_tests {
    use super::*;
    use crate::stream::payloads::{BlockCanonicityUpdatePayload, InternalCommandCanonicityPayload, InternalCommandPayload};
    use std::sync::atomic::Ordering;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_internal_command_canonicity_actor_processes_internal_command_updates() -> anyhow::Result<()> {
        let shared_publisher = Arc::new(SharedPublisher::new(200));
        let actor = InternalCommandCanonicityActor::new(Arc::clone(&shared_publisher));

        let mut receiver = shared_publisher.subscribe();

        let internal_command_payload = InternalCommandPayload {
            internal_command_type: InternalCommandType::Coinbase,
            height: 10,
            state_hash: "sample_hash".to_string(),
            timestamp: 123456,
            amount_nanomina: 100_000_000,
            recipient: "recipient_public_key".to_string(),
            source: None,
        };

        actor
            .handle_event(Event {
                event_type: EventType::InternalCommand,
                payload: sonic_rs::to_string(&internal_command_payload).unwrap(),
            })
            .await;

        let canonical_update_payload = BlockCanonicityUpdatePayload {
            height: 10,
            canonical: true,
            state_hash: "sample_hash".to_string(),
            was_canonical: false,
        };

        actor
            .handle_event(Event {
                event_type: EventType::BlockCanonicityUpdate,
                payload: sonic_rs::to_string(&canonical_update_payload).unwrap(),
            })
            .await;

        let published_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(published_event.is_ok(), "Expected an InternalCommandCanonicityUpdate event to be published.");

        if let Ok(Ok(event)) = published_event {
            let published_payload: InternalCommandCanonicityPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(published_payload.height, internal_command_payload.height);
            assert_eq!(published_payload.state_hash, internal_command_payload.state_hash);
            assert_eq!(published_payload.timestamp, internal_command_payload.timestamp);
            assert_eq!(published_payload.internal_command_type, internal_command_payload.internal_command_type);
            assert_eq!(published_payload.amount_nanomina, internal_command_payload.amount_nanomina);
            assert_eq!(published_payload.recipient, internal_command_payload.recipient);
            assert!(published_payload.canonical);
        }

        assert_eq!(actor.events_published.load(Ordering::SeqCst), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_internal_command_canonicity_actor_prunes_internal_commands_on_transition_frontier() -> anyhow::Result<()> {
        let shared_publisher = Arc::new(SharedPublisher::new(200));
        let actor = InternalCommandCanonicityActor::new(Arc::clone(&shared_publisher));

        {
            let mut internal_commands = actor.internal_commands.lock().await;
            internal_commands.insert(
                Height(5),
                vec![InternalCommandPayload {
                    internal_command_type: InternalCommandType::FeeTransfer,
                    height: 5,
                    state_hash: "hash_5".to_string(),
                    timestamp: 1000,
                    amount_nanomina: 5000,
                    recipient: "recipient_5".to_string(),
                    source: None,
                }],
            );
            internal_commands.insert(
                Height(10),
                vec![InternalCommandPayload {
                    internal_command_type: InternalCommandType::Coinbase,
                    height: 10,
                    state_hash: "hash_10".to_string(),
                    timestamp: 2000,
                    amount_nanomina: 10000,
                    recipient: "recipient_10".to_string(),
                    source: None,
                }],
            );
        }

        let transition_event = Event {
            event_type: EventType::TransitionFrontier,
            payload: sonic_rs::to_string(&7u64).unwrap(),
        };
        actor.handle_event(transition_event).await;

        {
            let internal_commands = actor.internal_commands.lock().await;
            assert!(
                !internal_commands.contains_key(&Height(5)),
                "InternalCommand with height 5 should have been pruned"
            );
            assert!(
                internal_commands.contains_key(&Height(10)),
                "InternalCommand with height 10 should not have been pruned"
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_internal_command_canonicity_actor_waits_for_internal_commands() -> anyhow::Result<()> {
        use crate::stream::payloads::{BlockCanonicityUpdatePayload, InternalCommandCanonicityPayload, InternalCommandPayload};
        use std::sync::atomic::Ordering;
        use tokio::time::{timeout, Duration};

        // Set up a shared publisher and instantiate the actor
        let shared_publisher = Arc::new(SharedPublisher::new(200));
        let actor = InternalCommandCanonicityActor::new(Arc::clone(&shared_publisher));

        // Subscribe to capture any output events from the actor
        let mut receiver = shared_publisher.subscribe();

        // Define a BlockCanonicityUpdatePayload and send it before adding InternalCommand
        let canonical_update_payload = BlockCanonicityUpdatePayload {
            height: 15,
            canonical: true,
            state_hash: "sample_hash_pending".to_string(),
            was_canonical: false,
        };

        actor
            .handle_event(Event {
                event_type: EventType::BlockCanonicityUpdate,
                payload: sonic_rs::to_string(&canonical_update_payload).unwrap(),
            })
            .await;

        // Attempt to receive an event with a timeout (should fail as the internal command is not yet available)
        let no_event = timeout(Duration::from_millis(100), receiver.recv()).await;
        assert!(no_event.is_err(), "Expected no event to be published since InternalCommand is not available.");

        // Now define the InternalCommandPayload and send it after the BlockCanonicityUpdate
        let internal_command_payload = InternalCommandPayload {
            internal_command_type: InternalCommandType::Coinbase,
            height: 15,
            state_hash: "sample_hash_pending".to_string(),
            timestamp: 123456,
            amount_nanomina: 100_000_000,
            recipient: "recipient_public_key".to_string(),
            source: None,
        };

        actor
            .handle_event(Event {
                event_type: EventType::InternalCommand,
                payload: sonic_rs::to_string(&internal_command_payload).unwrap(),
            })
            .await;

        // Now the InternalCommandCanonicityUpdate event should be published
        let published_event = timeout(Duration::from_secs(1), receiver.recv()).await;
        assert!(
            published_event.is_ok(),
            "Expected an InternalCommandCanonicityUpdate event to be published once both BlockCanonicityUpdate and InternalCommand are available."
        );

        if let Ok(Ok(event)) = published_event {
            let published_payload: InternalCommandCanonicityPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(published_payload.height, internal_command_payload.height);
            assert_eq!(published_payload.state_hash, internal_command_payload.state_hash);
            assert_eq!(published_payload.timestamp, internal_command_payload.timestamp);
            assert_eq!(published_payload.internal_command_type, internal_command_payload.internal_command_type);
            assert_eq!(published_payload.amount_nanomina, internal_command_payload.amount_nanomina);
            assert_eq!(published_payload.recipient, internal_command_payload.recipient);
            assert!(published_payload.canonical);
        }

        // Verify that events_published has been incremented
        assert_eq!(actor.events_published.load(Ordering::SeqCst), 1);

        Ok(())
    }
}
