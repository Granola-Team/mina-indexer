use super::payloads::BlockCanonicityUpdatePayload;
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};

pub trait CanonicalItem: Clone + Send + Sync {
    fn set_canonical(&mut self, canonical: bool);
    fn get_height(&self) -> u64;
    fn get_state_hash(&self) -> &str; // Add method to retrieve state hash
}

#[derive(Clone)]
pub struct CanonicalItemsManager<X>
where
    X: CanonicalItem,
{
    block_canonicity_updates: Arc<Mutex<HashMap<u64, VecDeque<BlockCanonicityUpdatePayload>>>>,
    expected_counts: Arc<Mutex<HashMap<u64, u64>>>,
    items: Arc<Mutex<HashMap<u64, Vec<X>>>>,
    prune_limit: u64,
}

impl<X> CanonicalItemsManager<X>
where
    X: CanonicalItem,
{
    pub fn new(prune_limit: u64) -> Self {
        Self {
            block_canonicity_updates: Arc::new(Mutex::new(HashMap::new())),
            expected_counts: Arc::new(Mutex::new(HashMap::new())),
            items: Arc::new(Mutex::new(HashMap::new())),
            prune_limit,
        }
    }

    /// Adds a block canonicity update for a specific height.
    pub fn add_block_canonicity_update(&self, update: BlockCanonicityUpdatePayload) {
        let mut updates = self.block_canonicity_updates.lock().unwrap();
        updates.entry(update.height).or_insert_with(VecDeque::new).push_back(update);
    }

    /// Sets the expected number of `X` at a specific height.
    pub fn add_items_count(&self, height: u64, count: u64) {
        let mut counts = self.expected_counts.lock().unwrap();
        counts.insert(height, count);
    }

    /// Adds an `X` instance to the items collection at its corresponding height.
    pub fn add_item(&self, x: X) {
        let height = x.get_height();
        let mut items = self.items.lock().unwrap();
        items.entry(height).or_insert_with(Vec::new).push(x);
    }

    /// Drains and processes items at the specified height.
    pub fn get_updates(&self, start_height: u64) -> Vec<X> {
        let mut updates = self.block_canonicity_updates.lock().unwrap();
        let mut items = self.items.lock().unwrap();
        let counts = self.expected_counts.lock().unwrap();

        let mut processed_items = Vec::new();

        for height in (0..=start_height).rev() {
            if let Some(queue) = updates.get_mut(&height) {
                while let Some(update) = queue.pop_front() {
                    if let Some(entries) = items.get_mut(&height) {
                        if let Some(&expected_count) = counts.get(&height) {
                            // Check if the total number of items matches the expected count
                            if entries.len() as u64 == expected_count {
                                // Filter items that match the current state hash
                                let matching_items: Vec<_> = entries.iter_mut().filter(|e| e.get_state_hash() == update.state_hash).collect();

                                // Only process matching items for this state hash
                                if !matching_items.is_empty() {
                                    for item in matching_items {
                                        item.set_canonical(update.canonical);
                                        processed_items.push(item.clone());
                                    }
                                }
                            } else {
                                // Re-queue update if not enough total items are available
                                queue.push_front(update);
                                break;
                            }
                        }
                    }
                }

                // Remove the queue if it's empty
                if queue.is_empty() {
                    updates.remove(&height);
                }
            }
        }

        processed_items
    }

    /// Prunes items below the transition frontier range.
    pub fn prune(&self) {
        let mut updates = self.block_canonicity_updates.lock().unwrap();
        let mut items = self.items.lock().unwrap();
        let mut counts = self.expected_counts.lock().unwrap();

        if let Some(&highest_height) = updates.keys().max() {
            if highest_height < self.prune_limit {
                return;
            }
            let prune_below = highest_height - self.prune_limit;

            updates.retain(|&height, _| height > prune_below);
            items.retain(|&height, _| height > prune_below);
            counts.retain(|&height, _| height > prune_below);
        }
    }
}

#[cfg(test)]
mod canonical_items_manager_tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct MockCanonicityItem {
        height: u64,
        state_hash: String,
        canonical: bool,
    }

    impl CanonicalItem for MockCanonicityItem {
        fn set_canonical(&mut self, canonical: bool) {
            self.canonical = canonical;
        }

        fn get_height(&self) -> u64 {
            self.height
        }

        fn get_state_hash(&self) -> &str {
            &self.state_hash
        }
    }

    #[test]
    fn test_add_block_canonicity_update() {
        let manager = CanonicalItemsManager::<MockCanonicityItem>::new(100);

        let update = BlockCanonicityUpdatePayload {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            canonical: true,
            was_canonical: false,
        };

        manager.add_block_canonicity_update(update.clone());

        let updates = manager.block_canonicity_updates.lock().unwrap();
        assert_eq!(updates.get(&10).unwrap().len(), 1);
        assert_eq!(updates.get(&10).unwrap()[0], update);
    }

    #[test]
    fn test_add_items_count() {
        let manager = CanonicalItemsManager::<MockCanonicityItem>::new(100);

        manager.add_items_count(10, 5);

        let counts = manager.expected_counts.lock().unwrap();
        assert_eq!(*counts.get(&10).unwrap(), 5);
    }

    #[test]
    fn test_add_item() {
        let manager = CanonicalItemsManager::<MockCanonicityItem>::new(100);

        let item = MockCanonicityItem {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            canonical: false,
        };

        manager.add_item(item.clone());

        let items = manager.items.lock().unwrap();
        assert_eq!(items.get(&10).unwrap().len(), 1);
        assert_eq!(items.get(&10).unwrap()[0], item);
    }

    #[test]
    fn test_get_updates_processes_matching_items() {
        let manager = CanonicalItemsManager::<MockCanonicityItem>::new(100);

        let item_1 = MockCanonicityItem {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            canonical: false,
        };

        let item_2 = MockCanonicityItem {
            height: 10,
            state_hash: "other_state_hash".to_string(),
            canonical: false,
        };

        manager.add_item(item_1.clone());
        manager.add_item(item_2.clone());
        manager.add_items_count(10, 2);

        let update = BlockCanonicityUpdatePayload {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            canonical: true,
            was_canonical: false,
        };

        manager.add_block_canonicity_update(update);

        let processed_items = manager.get_updates(10);

        assert_eq!(processed_items.len(), 1);
        assert_eq!(processed_items[0].state_hash, "state_hash_10");
        assert!(processed_items[0].canonical);

        let items = manager.items.lock().unwrap();
        assert!(!items.get(&10).unwrap().is_empty());
        assert!(!items.get(&10).unwrap()[1].canonical);
    }

    #[test]
    fn test_get_updates_requeues_unprocessed_updates() {
        let manager = CanonicalItemsManager::<MockCanonicityItem>::new(100);

        let item = MockCanonicityItem {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            canonical: false,
        };

        manager.add_item(item.clone());
        manager.add_items_count(10, 2); // Expected count is 2, but only 1 item is added

        let update = BlockCanonicityUpdatePayload {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            canonical: true,
            was_canonical: false,
        };

        manager.add_block_canonicity_update(update.clone());

        let processed_items = manager.get_updates(10);

        assert!(processed_items.is_empty());

        let updates = manager.block_canonicity_updates.lock().unwrap();
        assert_eq!(updates.get(&10).unwrap().len(), 1); // Update should still be in the queue
    }

    #[test]
    fn test_prune_removes_old_entries() {
        let prune_limit = 4; // Set the prune limit
        let manager = CanonicalItemsManager::<MockCanonicityItem>::new(prune_limit);

        // Add items and updates from height 1 to 5
        for height in 1..=5 {
            let item = MockCanonicityItem {
                height,
                state_hash: format!("state_hash_{}", height),
                canonical: false,
            };
            manager.add_item(item);
            manager.add_items_count(height, 1);

            let update = BlockCanonicityUpdatePayload {
                height,
                state_hash: format!("state_hash_{}", height),
                canonical: true,
                was_canonical: false,
            };
            manager.add_block_canonicity_update(update);
        }

        // Prune items below the prune threshold
        manager.prune();

        let updates = manager.block_canonicity_updates.lock().unwrap();
        let items = manager.items.lock().unwrap();
        let counts = manager.expected_counts.lock().unwrap();

        // Assert that height 1 is removed
        assert!(!updates.contains_key(&1), "Height 1 should be pruned from updates");
        assert!(!items.contains_key(&1), "Height 1 should be pruned from items");
        assert!(!counts.contains_key(&1), "Height 1 should be pruned from counts");

        // Assert that heights 2 through 5 remain
        for height in 2..=5 {
            assert!(updates.contains_key(&height), "Height {} should remain in updates", height);
            assert!(items.contains_key(&height), "Height {} should remain in items", height);
            assert!(counts.contains_key(&height), "Height {} should remain in counts", height);
        }
    }

    #[test]
    fn test_multiple_state_hashes_at_same_height() {
        let manager = CanonicalItemsManager::<MockCanonicityItem>::new(100);

        let item_1 = MockCanonicityItem {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            canonical: false,
        };

        let item_2 = MockCanonicityItem {
            height: 10,
            state_hash: "state_hash_10_other".to_string(),
            canonical: false,
        };

        manager.add_item(item_1.clone());
        manager.add_item(item_2.clone());
        manager.add_items_count(10, 2);

        let update = BlockCanonicityUpdatePayload {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            canonical: true,
            was_canonical: false,
        };

        manager.add_block_canonicity_update(update);

        let processed_items = manager.get_updates(10);

        assert_eq!(processed_items.len(), 1);
        assert_eq!(processed_items[0].state_hash, "state_hash_10");
        assert!(processed_items[0].canonical);

        let remaining_items = manager.items.lock().unwrap();
        assert!(remaining_items.get(&10).unwrap().iter().any(|item| item.state_hash == "state_hash_10_other"));
    }

    #[test]
    fn test_traversal_from_start_height_to_zero() {
        let manager = CanonicalItemsManager::<MockCanonicityItem>::new(100);

        // Add items at multiple heights, including height 0
        for height in 0..=10 {
            let item = MockCanonicityItem {
                height,
                state_hash: format!("state_hash_{}", height),
                canonical: false,
            };
            manager.add_item(item);
            manager.add_items_count(height, 1);

            let update = BlockCanonicityUpdatePayload {
                height,
                state_hash: format!("state_hash_{}", height),
                canonical: true,
                was_canonical: false,
            };
            manager.add_block_canonicity_update(update);
        }

        // Process updates starting from height 10
        let processed_items = manager.get_updates(10);

        // Validate that all items from height 10 down to 0 are processed
        assert_eq!(processed_items.len(), 11, "All items from height 10 to 0 should be processed");

        // Check that items are in descending order
        for (i, item) in processed_items.iter().enumerate() {
            let expected_height = 10 - i as u64; // Expected height in descending order
            assert_eq!(item.height, expected_height);
            assert_eq!(item.state_hash, format!("state_hash_{}", expected_height));
            assert!(item.canonical, "Item at height {} should be marked canonical", expected_height);
        }
    }
}
