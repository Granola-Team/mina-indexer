use crate::{
    constants::TRANSITION_FRONTIER_DISTANCE,
    event_sourcing::{
        actor_dag::{ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
        canonical_items_manager::CanonicalItemsManager,
        events::{Event, EventType},
        payloads::{BerkeleyBlockPayload, BlockCanonicityUpdatePayload, CanonicalBerkeleyBlockPayload},
    },
};
use std::sync::Arc;
use tokio::sync::{watch, Mutex};

pub struct CanonicalBerkeleyBlockActor;

const CANONICAL_MANAGER_KEY: &str = "canonical_manager";

impl ActorFactory for CanonicalBerkeleyBlockActor {
    fn create_actor(shutdown_rx: watch::Receiver<bool>) -> ActorNode {
        let mut actor_store = ActorStore::new();
        actor_store.insert(
            CANONICAL_MANAGER_KEY,
            CanonicalItemsManager::<CanonicalBerkeleyBlockPayload>::new(TRANSITION_FRONTIER_DISTANCE as u64),
        );

        ActorNodeBuilder::new(EventType::BlockCanonicityUpdate)
            .with_state(actor_store)
            .with_processor(|event, state: Arc<Mutex<ActorStore>>, _requeue| {
                Box::pin(async move {
                    let mut state = state.lock().await;
                    let manager: CanonicalItemsManager<CanonicalBerkeleyBlockPayload> = state.remove(CANONICAL_MANAGER_KEY).unwrap();

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
                                    event_type: EventType::CanonicalBerkeleyBlock,
                                    payload: sonic_rs::to_string(&item).unwrap(),
                                })
                                .collect()
                        }
                        EventType::BerkeleyBlock => {
                            let berkeley_block: BerkeleyBlockPayload = sonic_rs::from_str(&event.payload).unwrap();

                            manager.add_items_count(berkeley_block.height, &berkeley_block.state_hash, 1).await;

                            let canonical_block = CanonicalBerkeleyBlockPayload {
                                canonical: false, // Default placeholder, updated later
                                was_canonical: false,
                                block: BerkeleyBlockPayload {
                                    height: berkeley_block.height,
                                    state_hash: berkeley_block.state_hash.clone(),
                                    previous_state_hash: berkeley_block.previous_state_hash.clone(),
                                    last_vrf_output: berkeley_block.last_vrf_output.clone(),
                                    user_command_count: berkeley_block.user_commands.len(),
                                    zk_app_command_count: berkeley_block.zk_app_command_count,
                                    user_commands: berkeley_block.user_commands.clone(),
                                    zk_app_commands: berkeley_block.zk_app_commands.clone(),
                                    snark_work_count: berkeley_block.snark_work_count,
                                    snark_work: berkeley_block.snark_work.clone(),
                                    timestamp: berkeley_block.timestamp,
                                    coinbase_receiver: berkeley_block.coinbase_receiver.clone(),
                                    coinbase_reward_nanomina: berkeley_block.coinbase_reward_nanomina,
                                    global_slot_since_genesis: berkeley_block.global_slot_since_genesis,
                                    fee_transfer_via_coinbase: berkeley_block.fee_transfer_via_coinbase.clone(),
                                    fee_transfers: berkeley_block.fee_transfers.clone(),
                                },
                            };

                            manager.add_item(canonical_block).await;

                            // Process updates to publish canonical items
                            manager
                                .get_updates(berkeley_block.height)
                                .await
                                .into_iter()
                                .map(|item| Event {
                                    event_type: EventType::CanonicalBerkeleyBlock,
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
mod canonical_berkeley_block_actor_tests_v2 {
    use super::*;
    use crate::{
        constants::GENESIS_STATE_HASH,
        event_sourcing::{
            actor_dag::{ActorFactory, ActorNode},
            events::{Event, EventType},
            payloads::{BerkeleyBlockPayload, BlockCanonicityUpdatePayload, CanonicalBerkeleyBlockPayload},
        },
    };
    use std::sync::Arc;
    use tokio::sync::{watch, Mutex};

    #[tokio::test]
    async fn test_block_canonicity_update_first() {
        // Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Create the CanonicalBerkeleyBlockActor
        let mut actor = CanonicalBerkeleyBlockActor::create_actor(shutdown_rx);

        // Add a receiver for capturing CanonicalBerkeleyBlock events
        let mut receiver = actor.add_receiver(EventType::CanonicalBerkeleyBlock);

        // Wrap the actor in an Arc<Mutex> for shared ownership
        let actor = Arc::new(Mutex::new(actor));

        // Spawn the actor
        tokio::spawn({
            let actor_clone = Arc::clone(&actor);
            async move {
                ActorNode::spawn_all(actor_clone).await;
            }
        });

        let block_payload = BerkeleyBlockPayload {
            height: 100,
            state_hash: "state_hash_100".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "vrf_output".to_string(),
            user_command_count: 0,
            zk_app_command_count: 0,
            user_commands: vec![],
            zk_app_commands: vec![],
            snark_work_count: 0,
            snark_work: vec![],
            timestamp: 0,
            coinbase_receiver: "receiver".to_string(),
            coinbase_reward_nanomina: 1000,
            global_slot_since_genesis: 10,
            fee_transfer_via_coinbase: None,
            fee_transfers: vec![],
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
            "Unexpected event emitted before BerkeleyBlock"
        );

        // Send BerkeleyBlock
        actor
            .lock()
            .await
            .get_sender()
            .unwrap()
            .send(Event {
                event_type: EventType::BerkeleyBlock,
                payload: sonic_rs::to_string(&block_payload).unwrap(),
            })
            .await
            .expect("Failed to send BerkeleyBlock event");

        // Confirm the event is emitted now
        if let Ok(Some(received_event)) = tokio::time::timeout(tokio::time::Duration::from_secs(2), receiver.recv()).await {
            assert_eq!(received_event.event_type, EventType::CanonicalBerkeleyBlock);

            // Deserialize the payload and verify it matches expectations
            let payload: CanonicalBerkeleyBlockPayload = sonic_rs::from_str(&received_event.payload).unwrap();
            assert_eq!(payload.block.height, block_payload.height);
            assert_eq!(payload.block.state_hash, block_payload.state_hash);
            assert!(payload.canonical);
            assert!(!payload.was_canonical);
        } else {
            panic!("Expected CanonicalBerkeleyBlock event not received.");
        }

        // Signal shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_berkeley_block_first() {
        // Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Create the CanonicalBerkeleyBlockActor
        let mut actor = CanonicalBerkeleyBlockActor::create_actor(shutdown_rx);

        // Add a receiver for capturing CanonicalBerkeleyBlock events
        let mut receiver = actor.add_receiver(EventType::CanonicalBerkeleyBlock);

        // Wrap the actor in an Arc<Mutex> for shared ownership
        let actor = Arc::new(Mutex::new(actor));

        // Spawn the actor
        tokio::spawn({
            let actor_clone = Arc::clone(&actor);
            async move {
                ActorNode::spawn_all(actor_clone).await;
            }
        });

        let block_payload = BerkeleyBlockPayload {
            height: 100,
            state_hash: "state_hash_100".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "vrf_output".to_string(),
            user_command_count: 0,
            zk_app_command_count: 0,
            user_commands: vec![],
            zk_app_commands: vec![],
            snark_work_count: 0,
            snark_work: vec![],
            timestamp: 0,
            coinbase_receiver: "receiver".to_string(),
            coinbase_reward_nanomina: 1000,
            global_slot_since_genesis: 10,
            fee_transfer_via_coinbase: None,
            fee_transfers: vec![],
        };

        let canonicity_update = BlockCanonicityUpdatePayload {
            height: 100,
            state_hash: "state_hash_100".to_string(),
            canonical: true,
            was_canonical: false,
        };

        // Send BerkeleyBlock first
        actor
            .lock()
            .await
            .get_sender()
            .unwrap()
            .send(Event {
                event_type: EventType::BerkeleyBlock,
                payload: sonic_rs::to_string(&block_payload).unwrap(),
            })
            .await
            .expect("Failed to send BerkeleyBlock event");

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
            assert_eq!(received_event.event_type, EventType::CanonicalBerkeleyBlock);

            // Deserialize the payload and verify it matches expectations
            let payload: CanonicalBerkeleyBlockPayload = sonic_rs::from_str(&received_event.payload).unwrap();
            assert_eq!(payload.block.height, block_payload.height);
            assert_eq!(payload.block.state_hash, block_payload.state_hash);
            assert!(payload.canonical);
            assert!(!payload.was_canonical);
        } else {
            panic!("Expected CanonicalBerkeleyBlock event not received.");
        }

        // Signal shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }
}
