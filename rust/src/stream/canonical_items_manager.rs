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

    pub async fn get_len(&self) -> usize {
        let updates = self.block_canonicity_updates.lock().await;
        let items = self.items.lock().await;
        let counts = self.expected_counts.lock().await;
        updates.len() + items.len() + counts.len()
    }

    pub async fn add_block_canonicity_update(&self, update: BlockCanonicityUpdatePayload) {
        let mut updates = self.block_canonicity_updates.lock().await;
        updates.entry(update.height).or_insert_with(VecDeque::new).push_back(update);
    }

    pub async fn add_items_count(&self, height: u64, count: u64) {
        let mut counts = self.expected_counts.lock().await;
        counts.insert(height, count);
    }

    pub async fn add_item(&self, x: X) {
        let height = x.get_height();
        let mut items = self.items.lock().await;
        items.entry(height).or_insert_with(Vec::new).push(x);
    }

    pub async fn get_updates(&self, start_height: u64) -> Vec<X> {
        let mut updates = self.block_canonicity_updates.lock().await;
        let mut items = self.items.lock().await;
        let counts = self.expected_counts.lock().await;

        let mut processed_items = Vec::new();

        for height in (0..=start_height).rev() {
            if let Some(queue) = updates.get_mut(&height) {
                while let Some(update) = queue.pop_front() {
                    if let Some(entries) = items.get_mut(&height) {
                        if let Some(&expected_count) = counts.get(&height) {
                            if entries.len() as u64 == expected_count {
                                let matching_items: Vec<_> = entries.iter_mut().filter(|e| e.get_state_hash() == update.state_hash).collect();

                                if !matching_items.is_empty() {
                                    for item in matching_items {
                                        item.set_canonical(update.canonical);
                                        item.set_was_canonical(update.was_canonical);
                                        processed_items.push(item.clone());
                                    }
                                }
                            } else {
                                queue.push_front(update);
                                break;
                            }
                        }
                    }
                }

                if queue.is_empty() {
                    updates.remove(&height);
                }
            }
        }

        processed_items
    }

    pub async fn prune(&self) {
        let mut updates = self.block_canonicity_updates.lock().await;
        let mut items = self.items.lock().await;
        let mut counts = self.expected_counts.lock().await;

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

        fn set_was_canonical(&mut self, was_canonical: bool) {
            self.canonical = was_canonical;
        }
    }

    #[tokio::test]
    async fn test_add_block_canonicity_update() {
        let manager = CanonicalItemsManager::<MockCanonicityItem>::new(100);

        let update = BlockCanonicityUpdatePayload {
            height: 10,
            state_hash: "state_hash_10".to_string(),
            canonical: true,
            was_canonical: false,
        };

        manager.add_block_canonicity_update(update.clone()).await;

        let updates = manager.block_canonicity_updates.lock().await;
        assert_eq!(updates.get(&10).unwrap().len(), 1);
        assert_eq!(updates.get(&10).unwrap()[0], update);
    }

    #[tokio::test]
    async fn test_prune_removes_old_entries() {
        let manager = CanonicalItemsManager::<MockCanonicityItem>::new(4);

        for height in 1..=5 {
            let item = MockCanonicityItem {
                height,
                state_hash: format!("state_hash_{}", height),
                canonical: false,
            };
            manager.add_item(item).await;
            manager.add_items_count(height, 1).await;

            let update = BlockCanonicityUpdatePayload {
                height,
                state_hash: format!("state_hash_{}", height),
                canonical: true,
                was_canonical: false,
            };
            manager.add_block_canonicity_update(update).await;
        }

        manager.prune().await;

        let updates = manager.block_canonicity_updates.lock().await;
        let items = manager.items.lock().await;
        let counts = manager.expected_counts.lock().await;

        assert!(!updates.contains_key(&1));
        assert!(!items.contains_key(&1));
        assert!(!counts.contains_key(&1));

        for height in 2..=5 {
            assert!(updates.contains_key(&height));
            assert!(items.contains_key(&height));
            assert!(counts.contains_key(&height));
        }
    }
}
