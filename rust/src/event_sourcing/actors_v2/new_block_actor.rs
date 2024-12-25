use crate::{
    blockchain_tree::{BlockchainTree, Hash, Height, Node},
    constants::TRANSITION_FRONTIER_DISTANCE,
    event_sourcing::{
        actor_dag::{ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
        events::{Event, EventType},
        payloads::{BlockAncestorPayload, NewBlockPayload},
    },
};
use log::warn;
use std::sync::Arc;
use tokio::sync::{watch, Mutex};

pub struct NewBlockActor;

const BLOCKCHAIN_TREE_KEY: &str = "blockchain_tree";

impl ActorFactory for NewBlockActor {
    fn create_actor(shutdown_rx: watch::Receiver<bool>) -> ActorNode {
        let mut state = ActorStore::new();
        state.insert("blockchain_tree", BlockchainTree::new(TRANSITION_FRONTIER_DISTANCE));
        ActorNodeBuilder::new(EventType::BlockAncestor)
            .with_state(state)
            .with_processor(|event, state: Arc<Mutex<ActorStore>>, requeue| {
                Box::pin(async move {
                    let mut state = state.lock().await;
                    let mut blockchain_tree: BlockchainTree = state.remove(BLOCKCHAIN_TREE_KEY).unwrap();

                    let block_payload: BlockAncestorPayload = sonic_rs::from_str(&event.payload).unwrap();
                    let next_node = Node {
                        height: Height(block_payload.height),
                        state_hash: Hash(block_payload.state_hash.clone()),
                        previous_state_hash: Hash(block_payload.previous_state_hash.clone()),
                        last_vrf_output: block_payload.last_vrf_output.clone(),
                        ..Default::default()
                    };

                    if blockchain_tree.is_empty() {
                        blockchain_tree.set_root(next_node.clone()).unwrap();
                    } else if blockchain_tree.has_parent(&next_node) {
                        blockchain_tree.add_node(next_node).unwrap();
                    } else {
                        if let Err(err) = requeue.send(event).await {
                            warn!("Unable to requeue event: {err}");
                        }
                        return None;
                    }

                    // Publish the NewBlock event
                    let added_payload = NewBlockPayload {
                        height: block_payload.height,
                        state_hash: block_payload.state_hash,
                        previous_state_hash: block_payload.previous_state_hash,
                        last_vrf_output: block_payload.last_vrf_output,
                    };

                    blockchain_tree.prune_tree().unwrap();

                    state.insert(BLOCKCHAIN_TREE_KEY, blockchain_tree);

                    println!("{:#?}", added_payload);

                    Some(vec![Event {
                        event_type: EventType::NewBlock,
                        payload: sonic_rs::to_string(&added_payload).unwrap(),
                    }])
                })
            })
            .build(shutdown_rx)
    }
}

#[cfg(test)]
mod blockchain_tree_builder_actor_tests_v2 {
    use super::NewBlockActor;
    use crate::event_sourcing::{
        actor_dag::{ActorFactory, ActorNode},
        events::{Event, EventType},
        payloads::{BlockAncestorPayload, NewBlockPayload},
    };
    use std::sync::Arc;
    use tokio::sync::{watch, Mutex};

    #[tokio::test]
    async fn test_add_root_to_empty_tree() {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Create the actor
        let mut actor = NewBlockActor::create_actor(shutdown_rx);

        // Add a receiver to capture the output
        let mut receiver = actor.add_receiver(EventType::NewBlock);

        // Consume the sender
        let sender = actor.get_sender().unwrap();

        // Wrap the actor in Arc<Mutex> for shared ownership
        let actor = Arc::new(Mutex::new(actor));

        // Spawn the actor
        tokio::spawn({
            let actor_clone = Arc::clone(&actor);
            async move {
                ActorNode::spawn_all(actor_clone).await;
            }
        });

        // Create and send the root block
        let root_payload = BlockAncestorPayload {
            height: 1,
            state_hash: "3N8aRootHash".to_string(),
            previous_state_hash: "".to_string(),
            last_vrf_output: "".to_string(),
        };

        sender
            .send(Event {
                event_type: EventType::BlockAncestor,
                payload: sonic_rs::to_string(&root_payload).unwrap(),
            })
            .await
            .expect("Failed to send root block");

        // Verify the root block was added to the tree
        if let Some(received_event) = receiver.recv().await {
            assert_eq!(received_event.event_type, EventType::NewBlock);
            let payload: NewBlockPayload = sonic_rs::from_str(&received_event.payload).unwrap();
            assert_eq!(payload.height, 1);
            assert_eq!(payload.state_hash, root_payload.state_hash);
        } else {
            panic!("Expected a NewBlock event for the root block, but none was received.");
        }

        // Signal shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_add_node_with_existing_parent() {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Create the actor
        let mut actor = NewBlockActor::create_actor(shutdown_rx);

        // Add a receiver to capture the output
        let mut receiver = actor.add_receiver(EventType::NewBlock);

        // Clone the sender
        let sender = actor.get_sender().unwrap();

        // Wrap the actor in Arc<Mutex> for shared ownership
        let actor = Arc::new(Mutex::new(actor));

        // Spawn the actor
        tokio::spawn({
            let actor_clone = Arc::clone(&actor);
            async move {
                ActorNode::spawn_all(actor_clone).await;
            }
        });

        // Add the root block
        let root_payload = BlockAncestorPayload {
            height: 1,
            state_hash: "3N8aRootHash".to_string(),
            previous_state_hash: "".to_string(),
            last_vrf_output: "".to_string(),
        };

        sender
            .send(Event {
                event_type: EventType::BlockAncestor,
                payload: sonic_rs::to_string(&root_payload).unwrap(),
            })
            .await
            .expect("Failed to send root block");

        // Add a child block with an existing parent
        let child_payload = BlockAncestorPayload {
            height: 2,
            state_hash: "3N8aChildHash".to_string(),
            previous_state_hash: "3N8aRootHash".to_string(),
            last_vrf_output: "".to_string(),
        };

        sender
            .send(Event {
                event_type: EventType::BlockAncestor,
                payload: sonic_rs::to_string(&child_payload).unwrap(),
            })
            .await
            .expect("Failed to send child block");

        // Verify the child block was added to the tree
        for expected_height in 1..=2 {
            if let Some(received_event) = receiver.recv().await {
                assert_eq!(received_event.event_type, EventType::NewBlock);
                let payload: NewBlockPayload = sonic_rs::from_str(&received_event.payload).unwrap();
                assert_eq!(payload.height, expected_height);
            } else {
                panic!("Expected a NewBlock event for block at height {expected_height}, but none was received.");
            }
        }

        // Signal shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_requeue_unconnected_node() {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Create the actor
        let mut actor = NewBlockActor::create_actor(shutdown_rx);

        // Add a receiver to capture the output
        let mut receiver = actor.add_receiver(EventType::NewBlock);

        // Clone the sender
        let sender = actor.get_sender().unwrap();

        // Wrap the actor in Arc<Mutex> for shared ownership
        let actor = Arc::new(Mutex::new(actor));

        // Spawn the actor
        tokio::spawn({
            let actor_clone = Arc::clone(&actor);
            async move {
                ActorNode::spawn_all(actor_clone).await;
            }
        });

        // Step 1: Add the root block (height 1)
        let root_block = BlockAncestorPayload {
            height: 1,
            state_hash: "3N8aRootHash".to_string(),
            previous_state_hash: "".to_string(),
            last_vrf_output: "".to_string(),
        };

        sender
            .send(Event {
                event_type: EventType::BlockAncestor,
                payload: sonic_rs::to_string(&root_block).unwrap(),
            })
            .await
            .expect("Failed to send root block");

        // Verify that the NewBlock event is fired for the root block
        if let Some(received_event) = receiver.recv().await {
            assert_eq!(received_event.event_type, EventType::NewBlock);
            let payload: NewBlockPayload = sonic_rs::from_str(&received_event.payload).unwrap();
            assert_eq!(payload.height, root_block.height);
            assert_eq!(payload.state_hash, root_block.state_hash);
        } else {
            panic!("Expected a NewBlock event for the root block, but none was received.");
        }

        // Step 2: Add a block at height + 2 (unconnected block)
        let unconnected_block = BlockAncestorPayload {
            height: 3,
            state_hash: "3N8aUnconnectedHash".to_string(),
            previous_state_hash: "3N8aNonExistentParent".to_string(),
            last_vrf_output: "".to_string(),
        };

        sender
            .send(Event {
                event_type: EventType::BlockAncestor,
                payload: sonic_rs::to_string(&unconnected_block).unwrap(),
            })
            .await
            .expect("Failed to send unconnected block");

        // Verify that no NewBlock event is fired for the unconnected block
        assert!(
            tokio::time::timeout(tokio::time::Duration::from_millis(100), receiver.recv()).await.is_err(),
            "Expected no NewBlock event for the unconnected block"
        );

        // Signal shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }
}
