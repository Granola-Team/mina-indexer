use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::stream::payloads::*;
use anyhow::Result;
use async_trait::async_trait;
use futures::lock::Mutex;
use std::{
    collections::{HashMap, VecDeque},
    sync::{atomic::AtomicUsize, Arc},
};

pub struct CanonicalBlockLogActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
    pub block_canonicity_queue: Arc<Mutex<VecDeque<BlockCanonicityUpdatePayload>>>,
    pub blocks: Arc<Mutex<HashMap<u64, Vec<BlockLogPayload>>>>,
}

impl CanonicalBlockLogActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "CanonicalBlockLogActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
            block_canonicity_queue: Arc::new(Mutex::new(VecDeque::new())),
            blocks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn process_canonical_blocks(&self) -> Result<(), &'static str> {
        let mut queue = self.block_canonicity_queue.lock().await;

        while let Some(update) = queue.pop_front() {
            let blocks_map = self.blocks.lock().await;
            if let Some(blocks) = blocks_map.get(&update.height) {
                for block in blocks.iter().filter(|uc| uc.state_hash == update.state_hash) {
                    let payload = CanonicalBlockLogPayload {
                        height: block.height,
                        state_hash: block.state_hash.to_string(),
                        previous_state_hash: block.previous_state_hash.to_string(),
                        user_command_count: block.user_command_count,
                        snark_work_count: block.snark_work_count,
                        timestamp: block.timestamp,
                        coinbase_receiver: block.coinbase_receiver.to_string(),
                        coinbase_reward_nanomina: block.coinbase_reward_nanomina,
                        global_slot_since_genesis: block.global_slot_since_genesis,
                        last_vrf_output: block.last_vrf_output.to_string(),
                        is_berkeley_block: block.is_berkeley_block,
                        canonical: update.canonical,
                    };
                    self.publish(Event {
                        event_type: EventType::CanonicalBlockLog,
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
impl Actor for CanonicalBlockLogActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.events_published
    }

    async fn report(&self) {
        let blocks = self.blocks.lock().await;
        self.print_report("Blocks HashMap", blocks.len());
        drop(blocks);
        let canonicity = self.block_canonicity_queue.lock().await;
        self.print_report("Block Canonicity VecDeque", canonicity.len());
    }

    async fn handle_event(&self, event: Event) {
        match event.event_type {
            EventType::BlockCanonicityUpdate => {
                let mut queue = self.block_canonicity_queue.lock().await;
                queue.push_back(sonic_rs::from_str(&event.payload).unwrap());
                drop(queue);
                self.process_canonical_blocks().await.unwrap();
            }
            EventType::BlockLog => {
                let event_payload: BlockLogPayload = sonic_rs::from_str(&event.payload).unwrap();
                let mut blocks = self.blocks.lock().await;
                blocks.entry(event_payload.height).or_insert_with(Vec::new).push(event_payload);
                drop(blocks);
                self.process_canonical_blocks().await.unwrap();
            }
            EventType::TransitionFrontier => {
                let height: u64 = sonic_rs::from_str(&event.payload).unwrap();
                let mut blocks = self.blocks.lock().await;
                blocks.retain(|key, _| key > &height.saturating_sub(1000));
                drop(blocks);
            }
            _ => {}
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[cfg(test)]
mod canonical_block_log_actor_tests {
    use super::*;
    use crate::stream::events::{Event, EventType};
    use std::sync::Arc;
    use tokio::time::timeout;

    async fn setup_actor() -> (Arc<CanonicalBlockLogActor>, tokio::sync::broadcast::Receiver<Event>) {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = Arc::new(CanonicalBlockLogActor::new(Arc::clone(&shared_publisher)));
        let receiver = shared_publisher.subscribe();
        (actor, receiver)
    }

    #[tokio::test]
    async fn test_block_log_persistence() {
        let (actor, _) = setup_actor().await;

        // Send a BlockLog event
        let block_log_payload = BlockLogPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            previous_state_hash: "hash_0".to_string(),
            user_command_count: 10,
            snark_work_count: 5,
            timestamp: 1234567890,
            coinbase_receiver: "receiver".to_string(),
            coinbase_reward_nanomina: 1000,
            global_slot_since_genesis: 123,
            last_vrf_output: "vrf_output".to_string(),
            is_berkeley_block: true,
        };

        let block_log_event = Event {
            event_type: EventType::BlockLog,
            payload: sonic_rs::to_string(&block_log_payload).unwrap(),
        };

        actor.handle_event(block_log_event).await;

        // Confirm block was added to blocks map
        let blocks = actor.blocks.lock().await;
        let block_entry = blocks.get(&1).unwrap();
        assert_eq!(block_entry.len(), 1);
        assert_eq!(block_entry[0].state_hash, "hash_1");
    }

    #[tokio::test]
    async fn test_canonical_block_log() {
        let (actor, mut receiver) = setup_actor().await;

        // Add a BlockLog event
        let block_log_payload = BlockLogPayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            previous_state_hash: "hash_0".to_string(),
            user_command_count: 10,
            snark_work_count: 5,
            timestamp: 1234567890,
            coinbase_receiver: "receiver".to_string(),
            coinbase_reward_nanomina: 1000,
            global_slot_since_genesis: 123,
            last_vrf_output: "vrf_output".to_string(),
            is_berkeley_block: true,
        };

        let block_log_event = Event {
            event_type: EventType::BlockLog,
            payload: sonic_rs::to_string(&block_log_payload).unwrap(),
        };

        actor.handle_event(block_log_event).await;

        // Send a BlockCanonicityUpdate event
        let block_canonicity_update_payload = BlockCanonicityUpdatePayload {
            height: 1,
            state_hash: "hash_1".to_string(),
            canonical: true,
            was_canonical: false,
        };

        let block_canonicity_update_event = Event {
            event_type: EventType::BlockCanonicityUpdate,
            payload: sonic_rs::to_string(&block_canonicity_update_payload).unwrap(),
        };

        actor.handle_event(block_canonicity_update_event).await;

        // Confirm CanonicalBlockLog event was published
        let received_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(received_event.is_ok(), "Expected a CanonicalBlockLog event");

        let event = received_event.unwrap().unwrap();
        assert_eq!(event.event_type, EventType::CanonicalBlockLog);

        let canonical_payload: CanonicalBlockLogPayload = sonic_rs::from_str(&event.payload).unwrap();
        assert_eq!(canonical_payload.height, 1);
        assert_eq!(canonical_payload.state_hash, "hash_1");
        assert!(canonical_payload.canonical);
    }

    #[tokio::test]
    async fn test_transition_frontier_removal() {
        let (actor, _) = setup_actor().await;

        // Add a BlockLog event
        let block_log_payload = BlockLogPayload {
            height: 5,
            state_hash: "hash_5".to_string(),
            previous_state_hash: "hash_4".to_string(),
            user_command_count: 8,
            snark_work_count: 4,
            timestamp: 1234567891,
            coinbase_receiver: "receiver_5".to_string(),
            coinbase_reward_nanomina: 500,
            global_slot_since_genesis: 125,
            last_vrf_output: "vrf_output_5".to_string(),
            is_berkeley_block: false,
        };

        let block_log_event = Event {
            event_type: EventType::BlockLog,
            payload: sonic_rs::to_string(&block_log_payload).unwrap(),
        };

        actor.handle_event(block_log_event).await;

        // Send a TransitionFrontier event
        let transition_frontier_event = Event {
            event_type: EventType::TransitionFrontier,
            payload: sonic_rs::to_string(&6).unwrap(),
        };

        actor.handle_event(transition_frontier_event).await;

        // Validate that blocks with height <= 4 are removed
        let blocks = actor.blocks.lock().await;
        assert!(blocks.get(&5).is_none(), "Block at height 5 should be removed");
    }
}
