use super::payloads::BlockCanonicityUpdatePayload;
use futures::lock::Mutex;
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

pub trait CanonicalItem: Clone + Send + Sync {
    fn set_canonical(&mut self, canonical: bool);
    fn get_height(&self) -> u64;
    fn get_state_hash(&self) -> &str;
    fn set_was_canonical(&mut self, was_canonical: bool);
}

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct CompositeKey {
    pub height: u64,
    pub state_hash: String,
}

impl CompositeKey {
    pub fn new(height: u64, state_hash: impl Into<String>) -> Self {
        Self {
            height,
            state_hash: state_hash.into(),
        }
    }
}

#[derive(Clone)]
pub struct CanonicalItemsManager<X>
where
    X: CanonicalItem,
{
    block_canonicity_updates: Arc<Mutex<HashMap<CompositeKey, VecDeque<BlockCanonicityUpdatePayload>>>>,
    expected_counts: Arc<Mutex<HashMap<CompositeKey, u64>>>,
    items: Arc<Mutex<HashMap<CompositeKey, Vec<X>>>>,
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

    pub async fn report(&self, prefix: &str) {
        async fn log_collection<T>(prefix: &str, name: &str, collection: &Mutex<HashMap<CompositeKey, T>>) {
            let map = collection.lock().await;
            if let (Some(lowest_height), Some(max_height)) = (map.keys().map(|k| k.height).min(), map.keys().map(|k| k.height).max()) {
                println!(
                    "{}: Collection {} is of size {} and key range is ({},{})",
                    prefix,
                    name,
                    map.len(),
                    lowest_height,
                    max_height
                );
            }
        }

        log_collection(prefix, "BlockCanonicityUpdates", &self.block_canonicity_updates).await;
        log_collection(prefix, "ExpectedCounts", &self.expected_counts).await;
        log_collection(prefix, "Items", &self.items).await;
    }

    pub async fn get_len(&self) -> usize {
        let updates = self.block_canonicity_updates.lock().await;
        let items = self.items.lock().await;
        let counts = self.expected_counts.lock().await;
        updates.len() + items.len() + counts.len()
    }

    pub async fn add_block_canonicity_update(&self, update: BlockCanonicityUpdatePayload) {
        let mut updates = self.block_canonicity_updates.lock().await;
        let key = CompositeKey::new(update.height, &update.state_hash);
        updates.entry(key).or_insert_with(VecDeque::new).push_back(update);
    }

    pub async fn add_items_count(&self, height: u64, state_hash: &str, count: u64) {
        let mut counts = self.expected_counts.lock().await;
        let key = CompositeKey::new(height, state_hash);
        counts.insert(key, count);
    }

    pub async fn add_item(&self, x: X) {
        let key = CompositeKey::new(x.get_height(), x.get_state_hash());
        let mut items = self.items.lock().await;
        items.entry(key).or_insert_with(Vec::new).push(x);
    }

    pub async fn get_updates(&self, start_height: u64) -> Vec<X> {
        let mut updates = self.block_canonicity_updates.lock().await;
        let items = self.items.lock().await;
        let counts = self.expected_counts.lock().await;

        let mut processed_items = Vec::new();

        for height in (0..=start_height).rev() {
            let keys_to_remove: Vec<_> = updates
                .iter_mut()
                .filter(|(k, _)| k.height == height)
                .filter_map(|(key, queue)| {
                    let mut requeue_updates = VecDeque::new();

                    while let Some(update) = queue.pop_front() {
                        if let Some(entries) = items.get(key) {
                            if let Some(&expected_count) = counts.get(key) {
                                if entries.len() as u64 == expected_count {
                                    let matching_items: Vec<_> = entries.iter().collect();

                                    for item in matching_items {
                                        let mut cloned_item = item.clone();
                                        cloned_item.set_canonical(update.canonical);
                                        cloned_item.set_was_canonical(update.was_canonical);
                                        processed_items.push(cloned_item);
                                    }
                                } else {
                                    // Criteria not met; requeue the update
                                    requeue_updates.push_back(update);
                                }
                            } else {
                                // No count found; requeue the update
                                requeue_updates.push_back(update);
                            }
                        } else {
                            // No entries found; requeue the update
                            requeue_updates.push_back(update);
                        }
                    }

                    // Update the queue with remaining updates
                    *queue = requeue_updates;

                    // Mark the key for removal if the queue is now empty
                    if queue.is_empty() {
                        Some(key.clone())
                    } else {
                        None
                    }
                })
                .collect();

            // Remove all keys with empty queues
            for key in keys_to_remove {
                updates.remove(&key);
            }
        }

        processed_items
    }

    pub async fn prune(&self) {
        let mut updates = self.block_canonicity_updates.lock().await;
        let mut items = self.items.lock().await;
        let mut counts = self.expected_counts.lock().await;

        if let Some(highest_height) = updates.keys().map(|k| k.height).max() {
            if highest_height < self.prune_limit {
                return;
            }
            let prune_below = highest_height - self.prune_limit;

            updates.retain(|k, _| k.height > prune_below);
            items.retain(|k, _| k.height > prune_below);
            counts.retain(|k, _| k.height > prune_below);
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
        was_canonical: bool,
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

        fn set_was_canonical(&mut self, was_canonical: bool) {
            self.was_canonical = was_canonical;
        }
    }

    #[tokio::test]
    async fn test_update_tree_traversal_top_to_bottom_with_matching_criteria() {
        let manager = CanonicalItemsManager::<MockCanonicityItem>::new(100);

        // Add updates at multiple heights
        for height in 0..=10 {
            let update = BlockCanonicityUpdatePayload {
                height,
                state_hash: format!("state_hash_{}", height),
                canonical: true,
                was_canonical: false,
            };
            manager.add_block_canonicity_update(update).await;
        }

        // Add items only at odd heights, with sufficient counts at height 5
        for height in (1..=10).step_by(2) {
            let item = MockCanonicityItem {
                height,
                state_hash: format!("state_hash_{}", height),
                canonical: false,
                was_canonical: false,
            };
            manager.add_item(item.clone()).await;
            let expected_count = if height == 5 { 1 } else { 2 }; // Sufficient at height 5, insufficient otherwise
            manager.add_items_count(height, &format!("state_hash_{}", height), expected_count).await;
        }

        // Process updates starting from height 10
        let processed_items = manager.get_updates(10).await;

        // Validate that items from height 5 are processed
        let processed_item = processed_items
            .iter()
            .find(|item| item.height == 5)
            .expect("Expected an item from height 5 to be processed");
        assert_eq!(
            processed_item.state_hash, "state_hash_5",
            "Processed item should match the state hash for height 5"
        );
        assert!(processed_item.canonical, "Processed item at height 5 should be marked canonical");

        // Validate that heights without matching criteria are not processed
        for height in (0..=10).rev() {
            if height != 5 {
                assert!(
                    !processed_items.iter().any(|item| item.height == height),
                    "No items from height {} should be processed",
                    height
                );
            }
        }

        // Validate that all heights were traversed
        let remaining_updates = manager.block_canonicity_updates.lock().await;
        for height in 0..=10 {
            let composite_key = CompositeKey::new(height, format!("state_hash_{}", height));
            if height != 5 {
                assert!(
                    !remaining_updates.get(&composite_key).unwrap().is_empty(),
                    "Updates for height {} should remain in the queue",
                    height
                );
            } else {
                assert!(
                    !remaining_updates.contains_key(&composite_key),
                    "Updates for height {} should be removed after processing",
                    height
                );
            }
        }
    }

    #[tokio::test]
    async fn test_independent_processing_of_state_hashes_at_same_height() {
        let manager = CanonicalItemsManager::<MockCanonicityItem>::new(100);

        // Add two items with different state hashes at the same height
        let item_1 = MockCanonicityItem {
            height: 10,
            state_hash: "state_hash_1".to_string(),
            canonical: false,
            was_canonical: false,
        };
        let item_2 = MockCanonicityItem {
            height: 10,
            state_hash: "state_hash_2".to_string(),
            canonical: false,
            was_canonical: false,
        };

        manager.add_item(item_1.clone()).await;
        manager.add_item(item_2.clone()).await;

        // Set expected counts independently for each state hash
        manager.add_items_count(10, "state_hash_1", 1).await; // Exact count for state_hash_1
        manager.add_items_count(10, "state_hash_2", 2).await; // Insufficient count for state_hash_2

        // Add updates for both state hashes
        let update_1 = BlockCanonicityUpdatePayload {
            height: 10,
            state_hash: "state_hash_1".to_string(),
            canonical: true,
            was_canonical: false,
        };
        let update_2 = BlockCanonicityUpdatePayload {
            height: 10,
            state_hash: "state_hash_2".to_string(),
            canonical: true,
            was_canonical: false,
        };

        manager.add_block_canonicity_update(update_1).await;
        manager.add_block_canonicity_update(update_2).await;

        // Process updates
        let processed_items = manager.get_updates(10).await;

        // Validate that only item_1 was processed
        assert_eq!(processed_items.len(), 1, "Only one item should be processed");
        assert_eq!(processed_items[0].state_hash, "state_hash_1", "Processed item should belong to state_hash_1");
        assert!(processed_items[0].canonical, "Processed item should be marked canonical");

        // Validate that item_2 remains unprocessed
        let remaining_updates = manager.block_canonicity_updates.lock().await;
        assert!(remaining_updates.contains_key(&CompositeKey::new(10, "state_hash_2")));
    }
}
