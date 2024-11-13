use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    blockchain_tree::{BlockchainTree, Hash, Height, Node},
    constants::{POSTGRES_CONNECTION_STRING, TRANSITION_FRONTIER_DISTANCE},
    stream::payloads::*,
};
use anyhow::Result;
use async_trait::async_trait;
use futures::lock::Mutex;
use std::{
    collections::VecDeque,
    sync::{atomic::AtomicUsize, Arc},
};
use tokio_postgres::{Client, NoTls};

pub struct BlockSummaryPersistenceActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
    pub block_canonicity_queue: Arc<Mutex<VecDeque<BlockCanonicityUpdatePayload>>>,
    pub blockchain_tree: Arc<Mutex<BlockchainTree>>,
    pub client: Client,
}

impl BlockSummaryPersistenceActor {
    pub async fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        if let Ok((client, connection)) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls).await {
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {}", e);
                }
            });
            if let Err(e) = client
                .execute(
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
                        is_canonical BOOLEAN,
                        UNIQUE (height, state_hash)
                    );",
                    &[],
                )
                .await
            {
                println!("Unable to create block_summary table {:?}", e);
            }
            Self {
                id: "BlockSummaryPersistenceActor".to_string(),
                shared_publisher,
                client,
                events_published: AtomicUsize::new(0),
                block_canonicity_queue: Arc::new(Mutex::new(VecDeque::new())),
                blockchain_tree: Arc::new(Mutex::new(BlockchainTree::new(TRANSITION_FRONTIER_DISTANCE))),
            }
        } else {
            panic!("Unable to establish connection to database")
        }
    }

    async fn db_upsert(&self, summary: &BlockSummaryPayload, canonical: bool) -> Result<u64, &'static str> {
        let upsert_query = r#"
            INSERT INTO block_summary (
                height, state_hash, previous_state_hash, user_command_count, snark_work_count,
                timestamp, coinbase_receiver, coinbase_reward_nanomina, global_slot_since_genesis,
                last_vrf_output, is_berkeley_block, is_canonical
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (height, state_hash)
            DO UPDATE SET
                previous_state_hash = EXCLUDED.previous_state_hash,
                user_command_count = EXCLUDED.user_command_count,
                snark_work_count = EXCLUDED.snark_work_count,
                timestamp = EXCLUDED.timestamp,
                coinbase_receiver = EXCLUDED.coinbase_receiver,
                coinbase_reward_nanomina = EXCLUDED.coinbase_reward_nanomina,
                global_slot_since_genesis = EXCLUDED.global_slot_since_genesis,
                last_vrf_output = EXCLUDED.last_vrf_output,
                is_berkeley_block = EXCLUDED.is_berkeley_block,
                is_canonical = EXCLUDED.is_canonical
        "#;

        match self
            .client
            .execute(
                upsert_query,
                &[
                    &(summary.height as i64),
                    &summary.state_hash,
                    &summary.previous_state_hash,
                    &(summary.user_command_count as i32),
                    &(summary.snark_work_count as i32),
                    &(summary.timestamp as i64),
                    &summary.coinbase_receiver,
                    &(summary.coinbase_reward_nanomina as i64),
                    &(summary.global_slot_since_genesis as i64),
                    &summary.last_vrf_output,
                    &summary.is_berkeley_block,
                    &canonical,
                ],
            )
            .await
        {
            Err(_) => Err("Unable to upsert into block_summary table"),
            Ok(affected_rows) => Ok(affected_rows),
        }
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

                if let Ok(affected_rows) = self.db_upsert(&block_summary, update.canonical).await {
                    assert_eq!(affected_rows, 1)
                };

                // Prune the tree as needed
                tree.prune_tree().unwrap();
            } else {
                // If the node is not found, push the update to the front of the queue
                queue.push_front(update);
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
