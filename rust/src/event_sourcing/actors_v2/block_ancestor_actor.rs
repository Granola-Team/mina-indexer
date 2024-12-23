use crate::event_sourcing::{
    actor_dag::{ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
    events::{Event, EventType},
    payloads::{BerkeleyBlockPayload, BlockAncestorPayload, MainnetBlockPayload},
};
use sonic_rs::from_str;
use tokio::sync::watch;

pub struct BlockAncestorActor;

impl ActorFactory for BlockAncestorActor {
    fn create_actor(shutdown_rx: watch::Receiver<bool>) -> ActorNode {
        ActorNodeBuilder::new(EventType::BerkeleyBlock) // Node listens for BerkeleyBlock and MainnetBlock
            .with_state(ActorStore::new())
            .with_processor(|event, _state| {
                Box::pin(async move {
                    match event.event_type {
                        EventType::BerkeleyBlock => {
                            // Deserialize BerkeleyBlock payload
                            let block_payload: BerkeleyBlockPayload = from_str(&event.payload).unwrap();
                            let block_ancestor_payload = BlockAncestorPayload {
                                height: block_payload.height,
                                state_hash: block_payload.state_hash.clone(),
                                previous_state_hash: block_payload.previous_state_hash.clone(),
                                last_vrf_output: block_payload.last_vrf_output,
                            };
                            // Publish the BlockAncestor event
                            Some(Event {
                                event_type: EventType::BlockAncestor,
                                payload: sonic_rs::to_string(&block_ancestor_payload).unwrap(),
                            })
                        }
                        EventType::MainnetBlock => {
                            // Deserialize MainnetBlock payload
                            let block_payload: MainnetBlockPayload = from_str(&event.payload).unwrap();
                            let block_ancestor_payload = BlockAncestorPayload {
                                height: block_payload.height,
                                state_hash: block_payload.state_hash.clone(),
                                previous_state_hash: block_payload.previous_state_hash.clone(),
                                last_vrf_output: block_payload.last_vrf_output,
                            };
                            // Publish the BlockAncestor event
                            Some(Event {
                                event_type: EventType::BlockAncestor,
                                payload: sonic_rs::to_string(&block_ancestor_payload).unwrap(),
                            })
                        }
                        _ => None,
                    }
                })
            })
            .build(shutdown_rx)
    }
}

#[cfg(test)]
mod block_ancestor_actor_tests_v2 {
    use super::BlockAncestorActor;
    use crate::event_sourcing::{
        actor_dag::{ActorFactory, ActorNode},
        events::{Event, EventType},
        payloads::{BerkeleyBlockPayload, BlockAncestorPayload, MainnetBlockPayload},
    };
    use std::sync::Arc;
    use tokio::sync::{watch, Mutex};

    #[tokio::test]
    async fn test_block_ancestor_actor_with_berkeley_block() {
        // Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Create the actor
        let mut actor = BlockAncestorActor::create_actor(shutdown_rx);

        // Add a receiver to the actor to capture its output
        let mut test_receiver = actor.add_receiver(EventType::BlockAncestor);

        // Consume the sender once and hold a reference to it
        let sender = actor.consume_sender().unwrap();

        // Wrap the actor in Arc<Mutex> for shared ownership
        let actor = Arc::new(Mutex::new(actor));

        // Spawn the actor
        tokio::spawn({
            let actor_clone = Arc::clone(&actor);
            async move {
                ActorNode::spawn_all(actor_clone).await;
            }
        });

        // Define a BerkeleyBlockPayload for the test
        let berkeley_block_payload = BerkeleyBlockPayload {
            height: 89,
            state_hash: "3NKVkEwELHY9CmPYxf25pwsKZpPf161QVCiC3JwdsyQwCYyE3wNCrRjWON".to_string(),
            previous_state_hash: "3NKJarZEsMAHkcPfhGA72eyjWBXGHergBZEoTuGXWS7vWeq8D5wu".to_string(),
            last_vrf_output: "hu0nffAHwdL0CYQNAlabyiUlwNWhlbj0MwynpKLtAAA=".to_string(),
            ..Default::default()
        };

        // Create an Event with serialized BerkeleyBlockPayload
        let event = Event {
            event_type: EventType::BerkeleyBlock,
            payload: sonic_rs::to_string(&berkeley_block_payload).unwrap(),
        };

        // Send the event to the actor
        sender.send(event).await.expect("Failed to send event");

        // Verify the BlockAncestor event
        if let Some(received_event) = test_receiver.recv().await {
            assert_eq!(received_event.event_type, EventType::BlockAncestor);

            // Deserialize the payload and verify fields
            let payload: BlockAncestorPayload = sonic_rs::from_str(&received_event.payload).unwrap();
            assert_eq!(payload.height, 89);
            assert_eq!(payload.state_hash, "3NKVkEwELHY9CmPYxf25pwsKZpPf161QVCiC3JwdsyQwCYyE3wNCrRjWON");
            assert_eq!(payload.previous_state_hash, "3NKJarZEsMAHkcPfhGA72eyjWBXGHergBZEoTuGXWS7vWeq8D5wu");
        } else {
            panic!("Did not receive expected BlockAncestor event.");
        }

        // Signal shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_block_ancestor_actor_with_mainnet_block() {
        // Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Create the actor
        let mut actor = BlockAncestorActor::create_actor(shutdown_rx);

        // Add a receiver to the actor to capture its output
        let mut test_receiver = actor.add_receiver(EventType::BlockAncestor);

        // Consume the sender once and hold a reference to it
        let sender = actor.consume_sender().unwrap();

        // Wrap the actor in Arc<Mutex> for shared ownership
        let actor = Arc::new(Mutex::new(actor));

        // Spawn the actor
        tokio::spawn({
            let actor_clone = Arc::clone(&actor);
            async move {
                ActorNode::spawn_all(actor_clone).await;
            }
        });

        // Define a MainnetBlockPayload for the test
        let mainnet_block_payload = MainnetBlockPayload {
            height: 101,
            state_hash: "4MTNpwef32H67dHk9Mx25ZLpHfVz27QXECm8C4o5eyRa5LgJ1qLScCwpJM".to_string(),
            previous_state_hash: "4MPXcYhJY8URpwZxBEmv9C7kXf5h41PLXeX9GoTwFg3TuL2Q9zMn".to_string(),
            last_vrf_output: "WXPOLoGn9vE7HwqkE-K5bH4d3LmSPPJQcfoLsrTDkQA=".to_string(),
            ..Default::default()
        };

        // Create an Event with serialized MainnetBlockPayload
        let event = Event {
            event_type: EventType::MainnetBlock,
            payload: sonic_rs::to_string(&mainnet_block_payload).unwrap(),
        };

        // Send the event to the actor
        sender.send(event).await.expect("Failed to send event");

        // Verify the BlockAncestor event
        if let Some(received_event) = test_receiver.recv().await {
            assert_eq!(received_event.event_type, EventType::BlockAncestor);

            // Deserialize the payload and verify fields
            let payload: BlockAncestorPayload = sonic_rs::from_str(&received_event.payload).unwrap();
            assert_eq!(payload.height, 101);
            assert_eq!(payload.state_hash, "4MTNpwef32H67dHk9Mx25ZLpHfVz27QXECm8C4o5eyRa5LgJ1qLScCwpJM");
            assert_eq!(payload.previous_state_hash, "4MPXcYhJY8URpwZxBEmv9C7kXf5h41PLXeX9GoTwFg3TuL2Q9zMn");
        } else {
            panic!("Did not receive expected BlockAncestor event.");
        }

        // Signal shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }
}
