use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::stream::{
    models::{Height, LastVrfOutput, StateHash},
    payloads::{BerkeleyBlockPayload, BlockCanonicityUpdatePayload, NewBlockAddedPayload},
};
use async_trait::async_trait;
use futures::lock::Mutex;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CompoundCanonicalEntry {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
    pub last_vrf_output: String,
}

impl CompoundCanonicalEntry {
    fn sort_entries(entries: &mut Vec<CompoundCanonicalEntry>) {
        entries.sort_by(|a, b| {
            // First compare by last_vrf_output (descending)
            match b.last_vrf_output.cmp(&a.last_vrf_output) {
                std::cmp::Ordering::Equal => {
                    // If there's a tie, compare by state_hash (descending)
                    b.state_hash.cmp(&a.state_hash)
                }
                other => other,
            }
        });
    }

    pub fn divide_on_canonicity(mut entries: &mut Vec<CompoundCanonicalEntry>) -> Option<(CompoundCanonicalEntry, Vec<CompoundCanonicalEntry>)> {
        if entries.is_empty() {
            return None;
        }
        Self::sort_entries(&mut entries);
        Some((entries.first().cloned().unwrap(), entries.split_off(1)))
    }
}

pub struct BlockCanonicityActor {
    id: String,
    shared_publisher: Arc<SharedPublisher>,
    events_processed: AtomicUsize,
    blockchain_tree: Arc<Mutex<HashMap<Height, Vec<CompoundCanonicalEntry>>>>,
    last_vrf_outputs: Arc<Mutex<HashMap<StateHash, LastVrfOutput>>>,
}

impl BlockCanonicityActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "BlockCanonicityActor".to_string(),
            shared_publisher,
            events_processed: AtomicUsize::new(0),
            blockchain_tree: Arc::new(Mutex::new(HashMap::new())),
            last_vrf_outputs: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Actor for BlockCanonicityActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn events_processed(&self) -> &AtomicUsize {
        &self.events_processed
    }
    async fn handle_event(&self, event: Event) {
        match event.event_type {
            EventType::BerkeleyBlock => {
                let payload: BerkeleyBlockPayload = sonic_rs::from_str(&event.payload).unwrap();
                let mut last_vrf_outputs = self.last_vrf_outputs.lock().await;
                last_vrf_outputs
                    .entry(StateHash(payload.state_hash))
                    .or_insert(LastVrfOutput(payload.last_vrf_output));
            }
            EventType::BlockAddedToTree => {
                let current_block_payload: NewBlockAddedPayload = sonic_rs::from_str(&event.payload).unwrap();
                let last_vrf_outputs = self.last_vrf_outputs.lock().await;
                let state_hash_key = StateHash(current_block_payload.state_hash.clone());
                if !last_vrf_outputs.contains_key(&state_hash_key) {
                    // try again later
                    return self.publish(Event {
                        event_type: EventType::BlockAddedToTree,
                        payload: event.payload,
                    });
                }
                let last_vrf_output = last_vrf_outputs.get(&state_hash_key).unwrap();
                let key = Height(current_block_payload.height);
                let value = CompoundCanonicalEntry {
                    height: current_block_payload.height.clone(),
                    state_hash: current_block_payload.state_hash.clone(),
                    previous_state_hash: current_block_payload.previous_state_hash.clone(),
                    last_vrf_output: last_vrf_output.0.clone(),
                };

                // blocks are consumed in order as they are added to tree
                // so if we see other blocks at the same height, we must tie break
                let mut blockchain_tree = self.blockchain_tree.lock().await;
                if blockchain_tree.contains_key(&key) {
                    blockchain_tree.entry(key.clone()).or_insert_with(Vec::new).push(value);
                    let mut entries = blockchain_tree.get(&key).cloned().unwrap();
                    drop(blockchain_tree);
                    let (canonical_entry, _) = CompoundCanonicalEntry::divide_on_canonicity(&mut entries).unwrap();
                    if canonical_entry.state_hash != current_block_payload.state_hash {
                        // The current block is not canonical. We only need to publish an
                        // update for the current block as the canonicity of other blocks
                        // has not changed at this height.
                        let canonicity_payload = BlockCanonicityUpdatePayload {
                            height: current_block_payload.height,
                            state_hash: current_block_payload.state_hash,
                            canonical: false,
                        };

                        self.publish(Event {
                            event_type: EventType::BlockCanonicityUpdate,
                            payload: sonic_rs::to_string(&canonicity_payload).unwrap(),
                        });
                        self.incr_event_processed();
                    }
                } else {
                    blockchain_tree.entry(key.clone()).or_insert_with(Vec::new).push(value);
                    drop(blockchain_tree);
                }
            }
            _ => {}
        }
    }

    fn publish(&self, event: Event) {
        self.shared_publisher.publish(event);
    }
}

async fn find_ancestry_until_common_ancestor(
    blockchain_tree: &Arc<Mutex<HashMap<Height, Vec<CompoundCanonicalEntry>>>>,
    mut prior_block: CompoundCanonicalEntry,
    mut new_block: CompoundCanonicalEntry,
) -> Option<(Vec<CompoundCanonicalEntry>, Vec<CompoundCanonicalEntry>, CompoundCanonicalEntry)> {
    let mut prior_ancestry = vec![prior_block.clone()];
    let mut new_ancestry = vec![new_block.clone()];

    loop {
        // Check if both blocks share the same state_hash, indicating a common ancestor
        if prior_block.state_hash == new_block.state_hash {
            // Remove the last item (common ancestor) from both lineages
            prior_ancestry.pop();
            new_ancestry.pop();
            // Return lineages without the common ancestor and the common ancestor separately
            return Some((prior_ancestry, new_ancestry, prior_block));
        }

        // Lock the blockchain tree once for both ancestry checks
        let blockchain_tree = blockchain_tree.lock().await;

        // Step back the prior block's ancestry based on its height
        let prior_entries_opt = blockchain_tree.get(&Height(prior_block.height - 1)).cloned();
        let new_entries_opt = blockchain_tree.get(&Height(new_block.height - 1)).cloned();

        drop(blockchain_tree); // Drop the lock

        // Process prior block ancestry
        if let Some(prior_entries) = prior_entries_opt {
            if let Some(next_prior_block) = prior_entries.iter().find(|entry| entry.state_hash == prior_block.previous_state_hash) {
                prior_block = next_prior_block.clone();
                prior_ancestry.push(prior_block.clone());
            } else {
                return None; // No further ancestry for prior_block
            }
        } else {
            return None; // No further ancestry for prior_block
        }

        // Process new block ancestry
        if let Some(new_entries) = new_entries_opt {
            if let Some(next_new_block) = new_entries.iter().find(|entry| entry.state_hash == new_block.previous_state_hash) {
                new_block = next_new_block.clone();
                new_ancestry.push(new_block.clone());
            } else {
                return None; // No further ancestry for new_block
            }
        } else {
            return None; // No further ancestry for new_block
        }
    }
}

#[tokio::test]
async fn test_find_ancestry_until_common_ancestor_excluding_common() {
    let blockchain_tree: Arc<Mutex<HashMap<Height, Vec<CompoundCanonicalEntry>>>> = Arc::new(Mutex::new(HashMap::new()));

    // Define some test blocks
    let common_ancestor = CompoundCanonicalEntry {
        height: 1,
        state_hash: "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ".to_string(),
        previous_state_hash: "genesis".to_string(),
        last_vrf_output: "ancestor_vrf".to_string(),
    };
    let prior_block1 = CompoundCanonicalEntry {
        height: 2,
        state_hash: "3NKXkGZpYLHa6Aei1VUuHYeZnacHT1yGZaFFd8suXx8CoKjx5pPw".to_string(),
        previous_state_hash: "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ".to_string(),
        last_vrf_output: "prior_vrf".to_string(),
    };
    let prior_block2 = CompoundCanonicalEntry {
        height: 3,
        state_hash: "3NKXjGYqBLHa6Aei1VUuWZeXmacXT2yPZaFDd7suzx8CoKjx6pQw".to_string(),
        previous_state_hash: "3NKXkGZpYLHa6Aei1VUuHYeZnacHT1yGZaFFd8suXx8CoKjx5pPw".to_string(),
        last_vrf_output: "prior_vrf2".to_string(),
    };
    let new_block1 = CompoundCanonicalEntry {
        height: 2,
        state_hash: "3NKPmQXrBQu6Ae1TVVuJYiWbYcHJX4tGZaFCe7suY8CoKjx6wQz".to_string(),
        previous_state_hash: "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ".to_string(),
        last_vrf_output: "new_vrf".to_string(),
    };
    let new_block2 = CompoundCanonicalEntry {
        height: 3,
        state_hash: "3NKZnGXpDQu6Afe2VYuHYeYcjbHT5xGZaFDf7suL8QqJkx7pPz".to_string(),
        previous_state_hash: "3NKPmQXrBQu6Ae1TVVuJYiWbYcHJX4tGZaFCe7suY8CoKjx6wQz".to_string(),
        last_vrf_output: "new_vrf2".to_string(),
    };

    // Insert blocks into the blockchain tree at corresponding heights
    let mut tree = blockchain_tree.lock().await;
    tree.insert(Height(1), vec![common_ancestor.clone()]);
    tree.insert(Height(2), vec![prior_block1.clone(), new_block1.clone()]);
    tree.insert(Height(3), vec![prior_block2.clone()]);
    tree.insert(Height(4), vec![new_block2.clone()]);
    drop(tree);

    // Call the function to find ancestry paths until the common ancestor
    let result = find_ancestry_until_common_ancestor(&blockchain_tree, prior_block2.clone(), new_block2.clone()).await;

    // Expected ancestry paths
    let expected_prior_ancestry = vec![prior_block2.clone(), prior_block1.clone()];
    let expected_new_ancestry = vec![new_block2.clone(), new_block1.clone()];

    // Check if the result matches the expected ancestry paths
    assert!(result.is_some(), "Expected common ancestor but got None");
    let (prior_ancestry, new_ancestry, ancestor) = result.unwrap();

    assert_eq!(prior_ancestry, expected_prior_ancestry, "Prior ancestry path mismatch");
    assert_eq!(new_ancestry, expected_new_ancestry, "New ancestry path mismatch");
    assert_eq!(ancestor, common_ancestor, "Common ancestor mismatch");
}

#[cfg(test)]
mod compound_canonical_entry_tests {
    use super::*;

    #[test]
    fn test_divide_on_canonicity() {
        let mut entries = vec![
            CompoundCanonicalEntry {
                height: 2,
                state_hash: "abc".to_string(),
                previous_state_hash: "xyz".to_string(),
                last_vrf_output: "def".to_string(),
            },
            CompoundCanonicalEntry {
                height: 2,
                state_hash: "uvw".to_string(),
                previous_state_hash: "xyz".to_string(),
                last_vrf_output: "def".to_string(),
            },
            CompoundCanonicalEntry {
                height: 2,
                state_hash: "mno".to_string(),
                previous_state_hash: "xyz".to_string(),
                last_vrf_output: "ghi".to_string(),
            },
            CompoundCanonicalEntry {
                height: 2,
                state_hash: "xyz".to_string(),
                previous_state_hash: "xyz".to_string(),
                last_vrf_output: "abc".to_string(),
            },
        ];

        // Divide on canonicity
        let result = CompoundCanonicalEntry::divide_on_canonicity(&mut entries);

        // Expected canonical and non-canonical results
        let expected_canonical = CompoundCanonicalEntry {
            height: 2,
            state_hash: "mno".to_string(),
            previous_state_hash: "xyz".to_string(),
            last_vrf_output: "ghi".to_string(),
        };
        let expected_non_canonical = vec![
            CompoundCanonicalEntry {
                height: 2,
                state_hash: "uvw".to_string(),
                previous_state_hash: "xyz".to_string(),
                last_vrf_output: "def".to_string(),
            },
            CompoundCanonicalEntry {
                height: 2,
                state_hash: "abc".to_string(),
                previous_state_hash: "xyz".to_string(),
                last_vrf_output: "def".to_string(),
            },
            CompoundCanonicalEntry {
                height: 2,
                state_hash: "xyz".to_string(),
                previous_state_hash: "xyz".to_string(),
                last_vrf_output: "abc".to_string(),
            },
        ];

        // Check that the result is as expected
        assert!(result.is_some());
        let (canonical, non_canonical) = result.unwrap();

        assert_eq!(canonical, expected_canonical);
        assert_eq!(non_canonical, expected_non_canonical);
    }

    #[test]
    fn test_divide_on_canonicity_same_vrf_different_state_hash() {
        let mut entries = vec![
            CompoundCanonicalEntry {
                height: 2,
                state_hash: "xyz".to_string(),
                previous_state_hash: "uvw".to_string(),
                last_vrf_output: "same".to_string(),
            },
            CompoundCanonicalEntry {
                height: 2,
                state_hash: "abc".to_string(),
                previous_state_hash: "uvw".to_string(),
                last_vrf_output: "same".to_string(),
            },
            CompoundCanonicalEntry {
                height: 2,
                state_hash: "mno".to_string(),
                previous_state_hash: "uvw".to_string(),
                last_vrf_output: "same".to_string(),
            },
        ];

        // Divide on canonicity with entries having the same `last_vrf_output`
        let result = CompoundCanonicalEntry::divide_on_canonicity(&mut entries);

        // Expected order should select the entry with the lexicographically largest `state_hash` as canonical
        let expected_canonical = CompoundCanonicalEntry {
            height: 2,
            state_hash: "xyz".to_string(),
            previous_state_hash: "uvw".to_string(),
            last_vrf_output: "same".to_string(),
        };
        let expected_non_canonical = vec![
            CompoundCanonicalEntry {
                height: 2,
                state_hash: "mno".to_string(),
                previous_state_hash: "uvw".to_string(),
                last_vrf_output: "same".to_string(),
            },
            CompoundCanonicalEntry {
                height: 2,
                state_hash: "abc".to_string(),
                previous_state_hash: "uvw".to_string(),
                last_vrf_output: "same".to_string(),
            },
        ];

        // Verify the result
        assert!(result.is_some());
        let (canonical, non_canonical) = result.unwrap();

        assert_eq!(canonical, expected_canonical);
        assert_eq!(non_canonical, expected_non_canonical);
    }

    #[test]
    fn test_divide_on_canonicity_empty() {
        let mut entries: Vec<CompoundCanonicalEntry> = vec![];

        // Divide on canonicity for an empty list
        let result = CompoundCanonicalEntry::divide_on_canonicity(&mut entries);

        // Should return None for empty input
        assert!(result.is_none());
    }
}

#[tokio::test]
async fn test_block_rebroadcast_until_vrf_output_available() -> anyhow::Result<()> {
    use crate::stream::payloads::{BerkeleyBlockPayload, NewBlockAddedPayload};

    // Create a shared publisher
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = BlockCanonicityActor::new(Arc::clone(&shared_publisher));

    // Create a BlockAddedToTree event without the corresponding VRF data
    let state_hash = "block_state_hash".to_string();
    let previous_state_hash = "previous_state_hash".to_string();
    let event = Event {
        event_type: EventType::BlockAddedToTree,
        payload: sonic_rs::to_string(&NewBlockAddedPayload {
            height: 2,
            state_hash: state_hash.clone(),
            previous_state_hash: previous_state_hash.clone(),
        })
        .unwrap(),
    };

    // Subscribe to the shared publisher
    let mut receiver = shared_publisher.subscribe();

    // Invoke the actor with the BlockAddedToTree event
    actor.on_event(event.clone()).await;

    // Expect the event to be rebroadcast due to missing VRF data
    if let Ok(received_event) = receiver.recv().await {
        assert_eq!(received_event.event_type, EventType::BlockAddedToTree);
        let payload: NewBlockAddedPayload = sonic_rs::from_str(&received_event.payload).unwrap();
        assert_eq!(payload.state_hash, state_hash);
    } else {
        panic!("Did not receive expected rebroadcasted event.");
    }

    // Now add the VRF data by publishing a BerkeleyBlock event
    let vrf_event = Event {
        event_type: EventType::BerkeleyBlock,
        payload: sonic_rs::to_string(&BerkeleyBlockPayload {
            height: 2,
            state_hash: state_hash.clone(),
            previous_state_hash: previous_state_hash.clone(),
            last_vrf_output: "valid_vrf_output".to_string(),
        })
        .unwrap(),
    };

    // Publish the VRF data
    actor.on_event(vrf_event).await;

    // Re-process the BlockAddedToTree event to check that it is now handled correctly
    actor.on_event(event).await;

    Ok(())
}

#[tokio::test]
async fn test_non_canonical_block_with_vrf_info() -> anyhow::Result<()> {
    use crate::{
        constants::GENESIS_STATE_HASH,
        stream::payloads::{BerkeleyBlockPayload, BlockCanonicityUpdatePayload, NewBlockAddedPayload},
    };
    use std::sync::atomic::Ordering;

    // Create shared publisher
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = BlockCanonicityActor::new(Arc::clone(&shared_publisher));

    // Set up VRF info for blocks
    let vrf_info_canonical = BerkeleyBlockPayload {
        height: 2,
        state_hash: "canonical_hash".to_string(),
        last_vrf_output: "b_vrf_highest".to_string(),
        previous_state_hash: GENESIS_STATE_HASH.to_string(),
    };
    let vrf_info_non_canonical = BerkeleyBlockPayload {
        height: 2,
        state_hash: "non_canonical_hash".to_string(),
        last_vrf_output: "a_vrf_lower".to_string(),
        previous_state_hash: GENESIS_STATE_HASH.to_string(),
    };

    // Add VRF information as BerkeleyBlock events for each block
    actor
        .handle_event(Event {
            event_type: EventType::BerkeleyBlock,
            payload: sonic_rs::to_string(&vrf_info_canonical).unwrap(),
        })
        .await;
    actor
        .handle_event(Event {
            event_type: EventType::BerkeleyBlock,
            payload: sonic_rs::to_string(&vrf_info_non_canonical).unwrap(),
        })
        .await;

    // Create canonical and non-canonical block payloads at the same height
    let canonical_block_payload = NewBlockAddedPayload {
        height: 2,
        state_hash: "canonical_hash".to_string(),
        previous_state_hash: GENESIS_STATE_HASH.to_string(),
    };
    let non_canonical_block_payload = NewBlockAddedPayload {
        height: 2,
        state_hash: "non_canonical_hash".to_string(),
        previous_state_hash: GENESIS_STATE_HASH.to_string(),
    };

    // Subscribe to the shared publisher to capture the output
    let mut receiver = shared_publisher.subscribe();

    // Handle the canonical block event first
    actor
        .handle_event(Event {
            event_type: EventType::BlockAddedToTree,
            payload: sonic_rs::to_string(&canonical_block_payload).unwrap(),
        })
        .await;

    // Handle the non-canonical block event, which should trigger a non-canonical update
    actor
        .handle_event(Event {
            event_type: EventType::BlockAddedToTree,
            payload: sonic_rs::to_string(&non_canonical_block_payload).unwrap(),
        })
        .await;

    // Verify a single publish event occurs for the non-canonical block, marking it non-canonical
    if let Ok(received_event) = receiver.recv().await {
        assert_eq!(received_event.event_type, EventType::BlockCanonicityUpdate);

        // Deserialize the payload and check values
        let payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&received_event.payload).unwrap();
        assert_eq!(payload.height, 2);
        assert_eq!(payload.state_hash, "non_canonical_hash");
        assert!(!payload.canonical); // Ensure the non-canonical block is marked as non-canonical

        assert_eq!(actor.events_processed().load(Ordering::SeqCst), 1);
    } else {
        panic!("Expected a BlockCanonicityUpdate event but did not receive one.");
    }

    Ok(())
}
