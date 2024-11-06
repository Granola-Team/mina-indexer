use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::stream::payloads::{BerkeleyBlockPayload, BlockAncestorPayload};
use async_trait::async_trait;
use std::sync::Arc;

pub struct BlockAncestorActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
}

#[async_trait]
impl Actor for BlockAncestorActor {
    fn id(&self) -> String {
        self.id.clone()
    }
    async fn on_event(&self, event: Event) {
        match event.event_type {
            EventType::BerkeleyBlock => {
                let block_payload: BerkeleyBlockPayload =
                    sonic_rs::from_str(&event.payload).unwrap();
                let block_ancestor_payload = BlockAncestorPayload {
                    height: block_payload.height,
                    state_hash: block_payload.state_hash.clone(),
                    previous_state_hash: block_payload.previous_state_hash.clone(),
                };
                self.publish(Event {
                    event_type: EventType::BlockAncestor,
                    payload: sonic_rs::to_string(&block_ancestor_payload).unwrap(),
                });
            }
            _ => {}
        }
    }

    fn publish(&self, event: Event) {
        self.shared_publisher.publish(event);
    }
}

#[tokio::test]
async fn test_block_ancestor_actor_with_berkeley_block() -> anyhow::Result<()> {
    // Create shared publisher
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = BlockAncestorActor {
        id: "TestActor".to_string(),
        shared_publisher: Arc::clone(&shared_publisher),
    };

    // Define BerkeleyBlockPayload for the test
    let berkeley_block_payload = BerkeleyBlockPayload {
        height: 89,
        state_hash: "3NKVkEwELHY9CmPYxf25pwsKZpPf161QVCiC3JwdsyQwCYyE3wNCrRjWON".to_string(),
        previous_state_hash: "3NKJarZEsMAHkcPfhGA72eyjWBXGHergBZEoTuGXWS7vWeq8D5wu".to_string(),
    };

    // Create an Event with serialized BerkeleyBlockPayload
    let event = Event {
        event_type: EventType::BerkeleyBlock,
        payload: sonic_rs::to_string(&berkeley_block_payload).unwrap(),
    };

    // Subscribe to the shared publisher
    let mut receiver = shared_publisher.subscribe();

    // Invoke the actor with the BerkeleyBlock event
    actor.on_event(event).await;

    // Assert that the correct BlockAncestor event is published
    if let Ok(received_event) = receiver.recv().await {
        assert_eq!(received_event.event_type, EventType::BlockAncestor);

        // Deserialize the payload and check values
        let payload: BlockAncestorPayload = sonic_rs::from_str(&received_event.payload).unwrap();
        assert_eq!(payload.height, 89);
        assert_eq!(
            payload.state_hash,
            "3NKVkEwELHY9CmPYxf25pwsKZpPf161QVCiC3JwdsyQwCYyE3wNCrRjWON"
        );
        assert_eq!(
            payload.previous_state_hash,
            "3NKJarZEsMAHkcPfhGA72eyjWBXGHergBZEoTuGXWS7vWeq8D5wu"
        );
    } else {
        panic!("Did not receive expected event from actor.");
    }

    Ok(())
}

// #[tokio::test]
// async fn test_block_ancestor_actor_with_mainnet_block() -> anyhow::Result<()> {
//     use std::io::Write;

//     // Create shared publisher
//     let shared_publisher = Arc::new(SharedPublisher::new(200));
//     let actor = BlockAncestorActor {
//         id: "TestActor".to_string(),
//         shared_publisher: Arc::clone(&shared_publisher),
//     };

//     // Create a temporary file for the MainnetBlock JSON
//     let mut block_file = tempfile::Builder::new()
//         .prefix("mainnet-45-3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPwgf9yam") // Updated prefix
//         .suffix(".json")
//         .tempfile()?;
//     writeln!(
//         block_file,
//         r#"{{
//             "scheduled_time": "1615940848214",
//             "protocol_state": {{
//                 "previous_state_hash": "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw",
//                 "body": {{
//                     "genesis_state_hash": "3..."
//                 }}
//             }}
//         }}"#
//     )
//     .unwrap();

//     // Create event pointing to the temporary file
//     let event = Event {
//         event_type: EventType::MainnetBlockPath,
//         payload: block_file.path().to_str().unwrap().to_string(),
//     };

//     // Subscribe to the shared publisher
//     let mut receiver = shared_publisher.subscribe();

//     // Invoke the actor with the MainnetBlock event
//     actor.on_event(event).await;

//     // Assert that the correct BlockAncestor event is published
//     if let Ok(received_event) = receiver.recv().await {
//         assert_eq!(received_event.event_type, EventType::BlockAncestor);

//         // Deserialize the payload and check values
//         let payload: BlockAncestorPayload = sonic_rs::from_str(&received_event.payload).unwrap();
//         assert_eq!(payload.height, 45); // Ensure this matches extract_height_and_hash
//         assert!(payload
//             .state_hash
//             .contains("3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPwgf9yam"));
//         assert_eq!(
//             payload.previous_state_hash,
//             "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw"
//         );
//     } else {
//         panic!("Did not receive expected event from actor.");
//     }

//     Ok(())
// }
