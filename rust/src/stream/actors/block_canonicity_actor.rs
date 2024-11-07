use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::stream::{
    models::{Height, LastVrfOutput, StateHash},
    payloads::{BerkeleyBlockPayload, NewBlockAddedPayload},
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
                self.incr_event_processed();
            }
            _ => {}
        }
    }

    fn publish(&self, event: Event) {
        self.shared_publisher.publish(event);
    }
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

    assert_eq!(actor.events_processed().load(std::sync::atomic::Ordering::SeqCst), 0);

    // Re-process the BlockAddedToTree event to check that it is now handled correctly
    actor.on_event(event).await;

    assert_eq!(actor.events_processed().load(std::sync::atomic::Ordering::SeqCst), 1);

    Ok(())
}
