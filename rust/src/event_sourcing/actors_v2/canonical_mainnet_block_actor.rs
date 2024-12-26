use crate::{
    constants::TRANSITION_FRONTIER_DISTANCE,
    event_sourcing::{
        actor_dag::{ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
        canonical_items_manager::CanonicalItemsManager,
        events::{Event, EventType},
        payloads::{BlockCanonicityUpdatePayload, CanonicalMainnetBlockPayload, MainnetBlockPayload},
    },
};
use std::sync::Arc;
use tokio::sync::{watch, Mutex};

pub struct CanonicalMainnetBlockActor;

const CANONICAL_MANAGER_KEY: &str = "canonical_manager";

impl ActorFactory for CanonicalMainnetBlockActor {
    fn create_actor(shutdown_rx: watch::Receiver<bool>) -> ActorNode {
        let mut actor_store = ActorStore::new();
        actor_store.insert(
            CANONICAL_MANAGER_KEY,
            CanonicalItemsManager::<CanonicalMainnetBlockPayload>::new(TRANSITION_FRONTIER_DISTANCE as u64),
        );

        ActorNodeBuilder::new(EventType::BlockCanonicityUpdate)
            .with_state(actor_store)
            .with_processor(|event, state: Arc<Mutex<ActorStore>>, _requeue| {
                Box::pin(async move {
                    let mut state = state.lock().await;
                    let manager: CanonicalItemsManager<CanonicalMainnetBlockPayload> = state.remove(CANONICAL_MANAGER_KEY).unwrap();

                    let output_events = match event.event_type {
                        EventType::BlockCanonicityUpdate => {
                            let canonicity_update: BlockCanonicityUpdatePayload = sonic_rs::from_str(&event.payload).unwrap();

                            manager.add_block_canonicity_update(canonicity_update.clone()).await;

                            // Process updates to publish canonical items
                            manager
                                .get_updates(canonicity_update.height)
                                .await
                                .into_iter()
                                .map(|item| Event {
                                    event_type: EventType::CanonicalMainnetBlock,
                                    payload: sonic_rs::to_string(&item).unwrap(),
                                })
                                .collect()
                        }
                        EventType::MainnetBlock => {
                            let mainnet_block: MainnetBlockPayload = sonic_rs::from_str(&event.payload).unwrap();

                            manager.add_items_count(mainnet_block.height, &mainnet_block.state_hash, 1).await;

                            let canonical_block = CanonicalMainnetBlockPayload {
                                canonical: false, // Default placeholder, updated later
                                was_canonical: false,
                                block: MainnetBlockPayload {
                                    height: mainnet_block.height,
                                    state_hash: mainnet_block.state_hash.clone(),
                                    previous_state_hash: mainnet_block.previous_state_hash.clone(),
                                    last_vrf_output: mainnet_block.last_vrf_output.clone(),
                                    user_command_count: mainnet_block.user_commands.len(),
                                    internal_command_count: mainnet_block.internal_command_count,
                                    user_commands: mainnet_block.user_commands.clone(),
                                    snark_work_count: mainnet_block.snark_work_count,
                                    snark_work: mainnet_block.snark_work.clone(),
                                    timestamp: mainnet_block.timestamp,
                                    coinbase_receiver: mainnet_block.coinbase_receiver.clone(),
                                    coinbase_reward_nanomina: mainnet_block.coinbase_reward_nanomina,
                                    global_slot_since_genesis: mainnet_block.global_slot_since_genesis,
                                    fee_transfer_via_coinbase: mainnet_block.fee_transfer_via_coinbase.clone(),
                                    fee_transfers: mainnet_block.fee_transfers.clone(),
                                    global_slot: mainnet_block.global_slot,
                                },
                            };

                            manager.add_item(canonical_block).await;

                            // Process updates to publish canonical items
                            manager
                                .get_updates(mainnet_block.height)
                                .await
                                .into_iter()
                                .map(|item| Event {
                                    event_type: EventType::CanonicalMainnetBlock,
                                    payload: sonic_rs::to_string(&item).unwrap(),
                                })
                                .collect()
                        }
                        _ => vec![],
                    };

                    if let Err(err) = manager.prune().await {
                        log::error!("Prune error: {}", err);
                    }

                    state.insert(CANONICAL_MANAGER_KEY, manager);

                    if output_events.is_empty() {
                        None
                    } else {
                        Some(output_events)
                    }
                })
            })
            .build(shutdown_rx)
    }
}

#[cfg(test)]
mod canonical_block_actor_tests_v2 {
    use super::*;
    use crate::{
        constants::GENESIS_STATE_HASH,
        event_sourcing::{
            actor_dag::{ActorFactory, ActorNode},
            events::{Event, EventType},
            payloads::{BlockCanonicityUpdatePayload, CanonicalMainnetBlockPayload, MainnetBlockPayload},
        },
    };
    use std::sync::Arc;
    use tokio::sync::{watch, Mutex};

    #[tokio::test]
    async fn test_block_canonicity_update_first() {
        // Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Create the CanonicalBlockActor
        let mut actor = CanonicalMainnetBlockActor::create_actor(shutdown_rx);

        // Add a receiver for capturing CanonicalMainnetBlock events
        let mut receiver = actor.add_receiver(EventType::CanonicalMainnetBlock);

        // Wrap the actor in an Arc<Mutex> for shared ownership
        let actor = Arc::new(Mutex::new(actor));

        // Spawn the actor
        tokio::spawn({
            let actor_clone = Arc::clone(&actor);
            async move {
                ActorNode::spawn_all(actor_clone).await;
            }
        });

        let block_payload = MainnetBlockPayload {
            height: 100,
            state_hash: "state_hash_100".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "vrf_output".to_string(),
            user_command_count: 0,
            internal_command_count: 0,
            user_commands: vec![],
            snark_work_count: 0,
            snark_work: vec![],
            timestamp: 0,
            coinbase_receiver: "receiver".to_string(),
            coinbase_reward_nanomina: 1000,
            global_slot_since_genesis: 10,
            fee_transfer_via_coinbase: None,
            fee_transfers: vec![],
            global_slot: 0,
        };

        let canonicity_update = BlockCanonicityUpdatePayload {
            height: 100,
            state_hash: "state_hash_100".to_string(),
            canonical: true,
            was_canonical: false,
        };

        // Send BlockCanonicityUpdate first
        actor
            .lock()
            .await
            .get_sender()
            .unwrap()
            .send(Event {
                event_type: EventType::BlockCanonicityUpdate,
                payload: sonic_rs::to_string(&canonicity_update).unwrap(),
            })
            .await
            .expect("Failed to send BlockCanonicityUpdate event");

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Confirm no event is emitted yet
        assert!(
            tokio::time::timeout(tokio::time::Duration::from_millis(100), receiver.recv()).await.is_err(),
            "Unexpected event emitted before MainnetBlock"
        );

        // Send MainnetBlock
        actor
            .lock()
            .await
            .get_sender()
            .unwrap()
            .send(Event {
                event_type: EventType::MainnetBlock,
                payload: sonic_rs::to_string(&block_payload).unwrap(),
            })
            .await
            .expect("Failed to send MainnetBlock event");

        // Confirm the event is emitted now
        if let Ok(Some(received_event)) = tokio::time::timeout(tokio::time::Duration::from_secs(2), receiver.recv()).await {
            assert_eq!(received_event.event_type, EventType::CanonicalMainnetBlock);

            // Deserialize the payload and verify it matches expectations
            let payload: CanonicalMainnetBlockPayload = sonic_rs::from_str(&received_event.payload).unwrap();
            assert_eq!(payload.block.height, block_payload.height);
            assert_eq!(payload.block.state_hash, block_payload.state_hash);
            assert!(payload.canonical);
            assert!(!payload.was_canonical);
        } else {
            panic!("Expected CanonicalMainnetBlock event not received.");
        }

        // Signal shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_mainnet_block_first() {
        // Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Create the CanonicalBlockActor
        let mut actor = CanonicalMainnetBlockActor::create_actor(shutdown_rx);

        // Add a receiver for capturing CanonicalMainnetBlock events
        let mut receiver = actor.add_receiver(EventType::CanonicalMainnetBlock);

        // Wrap the actor in an Arc<Mutex> for shared ownership
        let actor = Arc::new(Mutex::new(actor));

        // Spawn the actor
        tokio::spawn({
            let actor_clone = Arc::clone(&actor);
            async move {
                ActorNode::spawn_all(actor_clone).await;
            }
        });

        let block_payload = MainnetBlockPayload {
            height: 100,
            state_hash: "state_hash_100".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "vrf_output".to_string(),
            user_command_count: 0,
            internal_command_count: 0,
            user_commands: vec![],
            snark_work_count: 0,
            snark_work: vec![],
            timestamp: 0,
            coinbase_receiver: "receiver".to_string(),
            coinbase_reward_nanomina: 1000,
            global_slot_since_genesis: 10,
            fee_transfer_via_coinbase: None,
            fee_transfers: vec![],
            global_slot: 0,
        };

        let canonicity_update = BlockCanonicityUpdatePayload {
            height: 100,
            state_hash: "state_hash_100".to_string(),
            canonical: true,
            was_canonical: false,
        };

        // Send MainnetBlock first
        actor
            .lock()
            .await
            .get_sender()
            .unwrap()
            .send(Event {
                event_type: EventType::MainnetBlock,
                payload: sonic_rs::to_string(&block_payload).unwrap(),
            })
            .await
            .expect("Failed to send MainnetBlock event");

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Confirm no event is emitted yet
        assert!(
            tokio::time::timeout(tokio::time::Duration::from_millis(100), receiver.recv()).await.is_err(),
            "Unexpected event emitted before BlockCanonicityUpdate"
        );

        // Send BlockCanonicityUpdate
        actor
            .lock()
            .await
            .get_sender()
            .unwrap()
            .send(Event {
                event_type: EventType::BlockCanonicityUpdate,
                payload: sonic_rs::to_string(&canonicity_update).unwrap(),
            })
            .await
            .expect("Failed to send BlockCanonicityUpdate event");

        // Confirm the event is emitted now
        if let Ok(Some(received_event)) = tokio::time::timeout(tokio::time::Duration::from_secs(2), receiver.recv()).await {
            assert_eq!(received_event.event_type, EventType::CanonicalMainnetBlock);

            // Deserialize the payload and verify it matches expectations
            let payload: CanonicalMainnetBlockPayload = sonic_rs::from_str(&received_event.payload).unwrap();
            assert_eq!(payload.block.height, block_payload.height);
            assert_eq!(payload.block.state_hash, block_payload.state_hash);
            assert!(payload.canonical);
            assert!(!payload.was_canonical);
        } else {
            panic!("Expected CanonicalMainnetBlock event not received.");
        }

        // Signal shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }
}
