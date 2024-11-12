use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    blockchain_tree::{BlockchainTree, Hash, Height, Node},
    constants::TRANSITION_FRONTIER_DISTANCE,
    get_db_connection,
    stream::payloads::*,
};
use anyhow::Result;
use async_trait::async_trait;
use futures::lock::Mutex;
use std::{
    collections::VecDeque,
    sync::{atomic::AtomicUsize, Arc},
};

pub struct BlockSummaryPersistenceActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
    pub block_canonicity_queue: Arc<Mutex<VecDeque<BlockCanonicityUpdatePayload>>>,
    pub blockchain_tree: Arc<Mutex<BlockchainTree>>,
}

impl BlockSummaryPersistenceActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        let db = get_db_connection().unwrap();
        if let Err(e) = db.execute_batch(
            "CREATE TABLE IF NOT EXISTS block_summary (
                height BIGINT,
                state_hash TEXT,
                previous_state_hash TEXT,
                user_command_count INTEGER,
                snark_work_count INTEGER,
                timestamp BIGINT,
                coinbase_receiver TEXT,
                coinbase_reward_nanomina BIGINT,
                global_slot_since_genesis BIGINT,
                last_vrf_output TEXT,
                is_berkeley_block BOOLEAN,
                is_canonical BOOLEAN
            );",
        ) {
            println!("Unable to create block_summary table {:?}", e);
        }
        Self {
            id: "BlockSummaryPersistenceActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
            block_canonicity_queue: Arc::new(Mutex::new(VecDeque::new())),
            blockchain_tree: Arc::new(Mutex::new(BlockchainTree::new(TRANSITION_FRONTIER_DISTANCE))),
        }
    }

    async fn db_upsert(&self, summary: &BlockSummaryPayload, canonical: bool) -> Result<()> {
        match get_db_connection() {
            Ok(db) => {
                let delete_query = r#"
                        DELETE FROM block_summary
                        WHERE height = ? AND state_hash = ?
                    "#;
                if let Err(e) = db.execute(delete_query, [&summary.height.to_string(), &summary.state_hash.to_string()]) {
                    println!("Unable to delete block summary entry: {:?}", e);
                }
                let insert_query = r#"
                        INSERT INTO block_summary (
                            height, state_hash, previous_state_hash, user_command_count, snark_work_count,
                            timestamp, coinbase_receiver, coinbase_reward_nanomina, global_slot_since_genesis,
                            last_vrf_output, is_berkeley_block, is_canonical
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#;
                if let Err(e) = db.execute(
                    insert_query,
                    [
                        &summary.height.to_string(),                    // height
                        &summary.state_hash.to_string(),                // state_hash
                        &summary.previous_state_hash.to_string(),       // previous_state_hash
                        &summary.user_command_count.to_string(),        // user_command_count
                        &summary.snark_work_count.to_string(),          // snark_work_count
                        &summary.timestamp.to_string(),                 // timestamp
                        &summary.coinbase_receiver.to_string(),         // coinbase_receiver
                        &summary.coinbase_reward_nanomina.to_string(),  // coinbase_reward_nanomina
                        &summary.global_slot_since_genesis.to_string(), // global_slot_since_genesis
                        &summary.last_vrf_output.to_string(),           // last_vrf_output
                        &summary.is_berkeley_block.to_string(),         // is_berkeley_block
                        &canonical.to_string(),
                    ],
                ) {
                    println!("Unable to insert block summary entry: {:?}", e);
                }
            }
            Err(e) => {
                println!("Unable to get database connection: {:?}", e);
            }
        }

        Ok(())
    }

    async fn upsert_block_summary(&self) -> Result<(), &'static str> {
        let mut queue = self.block_canonicity_queue.lock().await;
        // Continue looping until the queue is empty
        while let Some(update) = queue.pop_front() {
            let mut tree = self.blockchain_tree.lock().await;

            // Try to retrieve the node based on the update's height and state hash
            if let Some(node) = tree.get_node(Height(update.height), Hash(update.state_hash.clone())) {
                // Deserialize the metadata string to extract block summary
                let block_summary: BlockSummaryPayload = sonic_rs::from_str(&node.metadata_str.unwrap()).unwrap();

                self.db_upsert(&block_summary, update.canonical).await.unwrap();

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
impl Actor for BlockSummaryPersistenceActor {
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
                self.upsert_block_summary().await.unwrap();
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
                self.upsert_block_summary().await.unwrap();
            }
            _ => {}
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}
