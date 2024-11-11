use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    blockchain_tree::{BlockchainTree, Hash, Height, Node},
    constants::TRANSITION_FRONTIER_DISTANCE,
    stream::payloads::*,
};
use anyhow::Result;
use async_trait::async_trait;
use futures::lock::Mutex;
use std::{
    collections::VecDeque,
    sync::{atomic::AtomicUsize, Arc},
};

pub struct BlockRewardDoubleEntryActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
    pub block_canonicity_queue: Arc<Mutex<VecDeque<BlockCanonicityUpdatePayload>>>,
    pub blockchain_tree: Arc<Mutex<BlockchainTree>>,
}

impl BlockRewardDoubleEntryActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "BestBlockActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
            block_canonicity_queue: Arc::new(Mutex::new(VecDeque::new())),
            blockchain_tree: Arc::new(Mutex::new(BlockchainTree::new(TRANSITION_FRONTIER_DISTANCE))),
        }
    }
    async fn process_canonicity_queue(&self) -> Result<(), &'static str> {
        let mut queue = self.block_canonicity_queue.lock().await;
        // Continue looping until the queue is empty
        while let Some(update) = queue.pop_front() {
            let mut tree = self.blockchain_tree.lock().await;

            // Try to retrieve the node based on the update's height and state hash
            if let Some(node) = tree.get_node(Height(update.height), Hash(update.state_hash.clone())) {
                // Deserialize the metadata string to extract block summary
                let block_summary: BlockSummaryPayload = sonic_rs::from_str(&node.metadata_str.unwrap()).unwrap();

                // Determine the correct DoubleEntryPayload based on canonicity
                let de_update = if update.canonical {
                    DoubleEntryPayload::builder()
                        .entry_type(DoubleEntryType::BlockReward)
                        .lhs_entry(block_summary.coinbase_reward_nanomina, TxnType::Credit, block_summary.coinbase_receiver)
                        .add_rhs_entry(block_summary.coinbase_reward_nanomina, TxnType::Debit, "Reward Pool".to_string())
                        .build()
                        .unwrap()
                } else {
                    DoubleEntryPayload::builder()
                        .entry_type(DoubleEntryType::BlockReward)
                        .lhs_entry(block_summary.coinbase_reward_nanomina, TxnType::Debit, block_summary.coinbase_receiver)
                        .add_rhs_entry(block_summary.coinbase_reward_nanomina, TxnType::Credit, "Reward Pool".to_string())
                        .build()
                        .unwrap()
                };

                // Publish the DoubleEntryTxn event
                self.publish(Event {
                    event_type: EventType::DoubleEntryTxn,
                    payload: sonic_rs::to_string(&de_update).unwrap(),
                });

                // Prune the tree as needed
                tree.prune_tree().unwrap();
            } else {
                // If the node is not found, push the update back to the end of the queue
                queue.push_back(update);
                drop(queue);
                // Exit to avoid a busy loop if we keep finding no nodes
                break;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Actor for BlockRewardDoubleEntryActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn events_published(&self) -> &AtomicUsize {
        &self.events_published
    }
    async fn handle_event(&self, event: Event) {
        match event.event_type {
            EventType::BlockCanonicityUpdate => {
                let mut queue = self.block_canonicity_queue.lock().await;
                queue.push_back(sonic_rs::from_str(&event.payload).unwrap());
                drop(queue);
                self.process_canonicity_queue().await.unwrap();
            }
            EventType::BlockSummary => {
                let event_payload: BlockSummaryPayload = sonic_rs::from_str(&event.payload).unwrap();
                let mut tree = self.blockchain_tree.lock().await;
                let node = Node {
                    height: Height(event_payload.height),
                    state_hash: Hash(event_payload.state_hash),
                    previous_state_hash: Hash(event_payload.previous_state_hash),
                    last_vrf_output: event_payload.last_vrf_output,
                    metadata_str: Some(event.payload),
                };
                if node.height.0 == 1 {
                    tree.set_root(node).unwrap();
                } else {
                    tree.add_node(node).unwrap();
                }
                drop(tree);
                self.process_canonicity_queue().await.unwrap();
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
mod block_reward_double_entry_actor_tests {
    use super::*;
    use crate::{
        constants::GENESIS_STATE_HASH,
        stream::{
            events::Event,
            payloads::{BlockCanonicityUpdatePayload, BlockSummaryPayload, DoubleEntryPayload, DoubleEntryType, TxnType},
        },
    };
    use std::sync::atomic::Ordering;
    use tokio::time::{timeout, Duration};

    // Helper function to set up the genesis block as the root node for each test
    async fn setup_genesis_block(actor: &BlockRewardDoubleEntryActor) {
        let genesis_block = BlockSummaryPayload {
            height: 1,
            state_hash: GENESIS_STATE_HASH.to_string(),
            previous_state_hash: "".to_string(),
            last_vrf_output: "genesis_vrf_output".to_string(),
            coinbase_reward_nanomina: 500_000_000_000,
            coinbase_receiver: "genesis_receiver".to_string(),
            timestamp: 0,
            snark_work_count: 0,
            user_command_count: 0,
            global_slot_since_genesis: 1,
            is_berkeley_block: false,
        };
        let genesis_event = Event {
            event_type: EventType::BlockSummary,
            payload: sonic_rs::to_string(&genesis_block).unwrap(),
        };
        actor.handle_event(genesis_event).await;
    }

    #[tokio::test]
    async fn test_block_reward_actor_processes_canonical_block() -> anyhow::Result<()> {
        let shared_publisher = Arc::new(SharedPublisher::new(200));
        let actor = BlockRewardDoubleEntryActor::new(Arc::clone(&shared_publisher));
        let mut receiver = shared_publisher.subscribe();

        setup_genesis_block(&actor).await;

        println!("genesis block added");

        // Create BlockSummary event building on genesis block
        let block_summary_payload = BlockSummaryPayload {
            height: 2,
            state_hash: "state_hash_2".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "vrf_output".to_string(),
            coinbase_reward_nanomina: 500_000_000_000,
            coinbase_receiver: "receiver_account".to_string(),
            timestamp: 1,
            snark_work_count: 1,
            user_command_count: 1,
            global_slot_since_genesis: 2,
            is_berkeley_block: false,
        };
        let block_summary_event = Event {
            event_type: EventType::BlockSummary,
            payload: sonic_rs::to_string(&block_summary_payload).unwrap(),
        };
        actor.handle_event(block_summary_event).await;

        println!("block 2 added");

        // Create a canonical BlockCanonicityUpdate event for the new block
        let canonical_payload = BlockCanonicityUpdatePayload {
            height: 2,
            state_hash: "state_hash_2".to_string(),
            canonical: true,
        };
        let canonicity_event = Event {
            event_type: EventType::BlockCanonicityUpdate,
            payload: sonic_rs::to_string(&canonical_payload).unwrap(),
        };

        // Process the canonical event
        actor.handle_event(canonicity_event).await;
        println!("canonicity event added");

        // Verify published DoubleEntry event with timeout
        let result = timeout(Duration::from_secs(2), receiver.recv()).await;
        match result {
            Ok(Ok(received_event)) => {
                assert_eq!(received_event.event_type, EventType::DoubleEntryTxn);
                let published_payload: DoubleEntryPayload = sonic_rs::from_str(&received_event.payload).unwrap();

                assert_eq!(published_payload.entry_type, DoubleEntryType::BlockReward);
                assert_eq!(published_payload.lhs_entry.value, 500_000_000_000);
                assert_eq!(published_payload.lhs_entry.txn_type, TxnType::Credit);
                assert_eq!(published_payload.lhs_entry.account, "receiver_account".to_string());
                assert_eq!(published_payload.rhs_entry[0].value, 500_000_000_000);
                assert_eq!(published_payload.rhs_entry[0].txn_type, TxnType::Debit);
                assert_eq!(published_payload.rhs_entry[0].account, "Reward Pool".to_string());
            }
            _ => panic!("Expected a DoubleEntryTxn event but did not receive one."),
        }

        assert_eq!(actor.events_published.load(Ordering::SeqCst), 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_block_reward_actor_processes_non_canonical_block() -> anyhow::Result<()> {
        let shared_publisher = Arc::new(SharedPublisher::new(200));
        let actor = BlockRewardDoubleEntryActor::new(Arc::clone(&shared_publisher));
        let mut receiver = shared_publisher.subscribe();

        setup_genesis_block(&actor).await;

        // Add a BlockSummary event building on the genesis block
        let block_summary_payload = BlockSummaryPayload {
            height: 2,
            state_hash: "state_hash_2".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "vrf_output".to_string(),
            coinbase_reward_nanomina: 500_000_000_000,
            coinbase_receiver: "receiver_account".to_string(),
            timestamp: 1,
            snark_work_count: 1,
            user_command_count: 1,
            global_slot_since_genesis: 2,
            is_berkeley_block: false,
        };
        let block_summary_event = Event {
            event_type: EventType::BlockSummary,
            payload: sonic_rs::to_string(&block_summary_payload).unwrap(),
        };
        actor.handle_event(block_summary_event).await;

        // Create a non-canonical BlockCanonicityUpdate event
        let non_canonical_payload = BlockCanonicityUpdatePayload {
            height: 2,
            state_hash: "state_hash_2".to_string(),
            canonical: false,
        };
        let canonicity_event = Event {
            event_type: EventType::BlockCanonicityUpdate,
            payload: sonic_rs::to_string(&non_canonical_payload).unwrap(),
        };

        // Process the non-canonical event
        actor.handle_event(canonicity_event).await;

        // Verify published DoubleEntry event with timeout
        let result = timeout(Duration::from_secs(2), receiver.recv()).await;
        match result {
            Ok(Ok(received_event)) => {
                assert_eq!(received_event.event_type, EventType::DoubleEntryTxn);
                let published_payload: DoubleEntryPayload = sonic_rs::from_str(&received_event.payload).unwrap();

                assert_eq!(published_payload.entry_type, DoubleEntryType::BlockReward);
                assert_eq!(published_payload.lhs_entry.value, 500_000_000_000);
                assert_eq!(published_payload.lhs_entry.txn_type, TxnType::Debit);
                assert_eq!(published_payload.lhs_entry.account, "receiver_account".to_string());
                assert_eq!(published_payload.rhs_entry[0].value, 500_000_000_000);
                assert_eq!(published_payload.rhs_entry[0].txn_type, TxnType::Credit);
                assert_eq!(published_payload.rhs_entry[0].account, "Reward Pool".to_string());
            }
            _ => panic!("Expected a DoubleEntryTxn event but did not receive one."),
        }

        assert_eq!(actor.events_published.load(Ordering::SeqCst), 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_block_reward_actor_no_update_if_block_not_found_then_applied_when_block_received() {
        let shared_publisher = Arc::new(SharedPublisher::new(200));
        let actor = BlockRewardDoubleEntryActor::new(Arc::clone(&shared_publisher));
        let mut receiver = shared_publisher.subscribe();

        setup_genesis_block(&actor).await;

        println!("add genesis block");

        // Create a BlockCanonicityUpdate event for a non-existent block (simulating a backlogged event)
        let missing_block_payload = BlockCanonicityUpdatePayload {
            height: 2,
            state_hash: "missing_block_hash".to_string(),
            canonical: true,
        };
        let canonicity_event = Event {
            event_type: EventType::BlockCanonicityUpdate,
            payload: sonic_rs::to_string(&missing_block_payload).unwrap(),
        };

        // Process the event (should not result in a published event because the block doesn't exist)
        actor.handle_event(canonicity_event).await;
        println!("canonicity update received");
        assert!(
            timeout(Duration::from_secs(1), receiver.recv()).await.is_err(),
            "No event should have been published for a non-existent block."
        );
        println!("no update issued");

        // Now add the previously missing block to the tree
        let missing_block_summary = BlockSummaryPayload {
            height: 2,
            state_hash: "missing_block_hash".to_string(),
            previous_state_hash: GENESIS_STATE_HASH.to_string(),
            last_vrf_output: "vrf_output".to_string(),
            coinbase_reward_nanomina: 500_000_000_000,
            coinbase_receiver: "receiver_account".to_string(),
            timestamp: 1,
            snark_work_count: 1,
            user_command_count: 1,
            global_slot_since_genesis: 2,
            is_berkeley_block: false,
        };
        let block_summary_event = Event {
            event_type: EventType::BlockSummary,
            payload: sonic_rs::to_string(&missing_block_summary).unwrap(),
        };
        actor.handle_event(block_summary_event).await;
        println!("canonicity update received");

        // Verify the previously queued canonicity update is now applied
        let result = timeout(Duration::from_secs(2), receiver.recv()).await;
        match result {
            Ok(Ok(received_event)) => {
                assert_eq!(received_event.event_type, EventType::DoubleEntryTxn);
                let published_payload: DoubleEntryPayload = sonic_rs::from_str(&received_event.payload).unwrap();

                assert_eq!(published_payload.entry_type, DoubleEntryType::BlockReward);
                assert_eq!(published_payload.lhs_entry.value, 500_000_000_000);
                assert_eq!(published_payload.lhs_entry.txn_type, TxnType::Credit);
                assert_eq!(published_payload.lhs_entry.account, "receiver_account".to_string());
                assert_eq!(published_payload.rhs_entry[0].value, 500_000_000_000);
                assert_eq!(published_payload.rhs_entry[0].txn_type, TxnType::Debit);
                assert_eq!(published_payload.rhs_entry[0].account, "Reward Pool".to_string());
            }
            _ => panic!("Expected a DoubleEntryTxn event but did not receive one after the block was added."),
        }

        assert_eq!(actor.events_published.load(Ordering::SeqCst), 1);
    }
}
