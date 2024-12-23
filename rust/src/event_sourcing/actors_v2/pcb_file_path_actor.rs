use crate::{
    event_sourcing::{
        actor_dag::{ActorFactory, ActorNode, ActorNodeBuilder, Stateless},
        events::{Event, EventType},
    },
    utility::get_top_level_keys_from_json_file,
};
use std::sync::Arc;
use tokio::sync::{watch::Receiver, Mutex};

pub struct PcbFilePathActor;

impl ActorFactory for PcbFilePathActor {
    type State = Stateless;

    fn create_actor(shutdown_rx: Receiver<bool>) -> Arc<Mutex<ActorNode<Self::State>>> {
        Arc::new(Mutex::new(
            ActorNodeBuilder::new(EventType::PrecomputedBlockPath)
                .with_state(Stateless {})
                .with_processor(|event, _state| {
                    Box::pin(async move {
                        let keys = get_top_level_keys_from_json_file(&event.payload).expect("file to exist");
                        if keys == vec!["data".to_string(), "version".to_string()] {
                            Some(Event {
                                event_type: EventType::BerkeleyBlockPath,
                                payload: event.payload,
                            })
                        } else {
                            Some(Event {
                                event_type: EventType::MainnetBlockPath,
                                payload: event.payload,
                            })
                        }
                    })
                })
                .build(shutdown_rx),
        ))
    }
}

#[cfg(test)]
mod pcb_file_path_actor_tests_v2 {
    use super::PcbFilePathActor;
    use crate::event_sourcing::{
        actor_dag::{ActorFactory, ActorNode},
        events::{Event, EventType},
    };

    #[tokio::test]
    async fn test_actor_with_add_receiver() {
        use std::sync::Arc;
        use tempfile::NamedTempFile;
        use tokio::sync::watch;

        // Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Create the actor
        let actor = PcbFilePathActor::create_actor(shutdown_rx);

        // Add a receiver to the actor
        let mut test_receiver = {
            let mut locked_actor = actor.lock().await;
            locked_actor.add_receiver(EventType::Test)
        };

        // Spawn the actor
        tokio::spawn({
            let actor_clone = Arc::clone(&actor);
            async move { ActorNode::spawn_all(actor_clone).await }
        });

        // Scenario: File with "data" and "version" keys (should trigger BerkeleyBlockPath)
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), r#"{"data": {}, "version": "1.0"}"#).unwrap();
        let test_event = Event {
            event_type: EventType::PrecomputedBlockPath,
            payload: temp_file.path().to_str().unwrap().to_string(),
        };

        // Send the event to the actor
        {
            let actor_sender = actor.lock().await.consume_sender().unwrap();
            actor_sender.send(test_event).await.expect("Failed to send event");
        }

        // Verify the event is sent through the test receiver
        if let Some(received_event) = test_receiver.recv().await {
            assert_eq!(received_event.event_type, EventType::BerkeleyBlockPath);
            assert_eq!(received_event.payload, temp_file.path().to_str().unwrap().to_string());
        } else {
            panic!("Did not receive expected BerkeleyBlockPath event.");
        }

        // Signal shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }
}
