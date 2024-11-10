use std::collections::BTreeMap;

#[derive(PartialOrd, Ord, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Height(pub u64);

#[derive(PartialOrd, Ord, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Hash(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Node {
    pub height: Height,
    pub state_hash: Hash,
    pub previous_state_hash: Hash,
    pub last_vrf_output: String,
    pub metadata_str: Option<String>,
}

#[derive(Debug)]
pub struct BlockchainTree {
    tree: BTreeMap<Height, Vec<Node>>,
    max_ancestors_from_best_tip: usize,
}

impl BlockchainTree {
    pub fn new(max_ancestors_from_best_tip: usize) -> Self {
        BlockchainTree {
            tree: BTreeMap::new(),
            max_ancestors_from_best_tip,
        }
    }

    pub fn get_node(&self, height: Height, state_hash: Hash) -> Option<Node> {
        self.tree
            .get(&height)
            .and_then(|entries| entries.into_iter().find(|entry| entry.state_hash == state_hash))
            .cloned()
    }

    pub fn set_root(&mut self, node: Node) -> Result<(), &'static str> {
        self.tree.entry(node.height.clone()).or_default().push(node);
        Ok(())
    }

    pub fn prune_tree(&mut self) -> Result<(), &'static str> {
        // Determine the best tip in the tree
        let (best_tip_height, _) = self.get_best_tip()?;

        // Calculate the minimum height allowed based on the max_ancestors_from_best_tip
        let min_allowed_height = if best_tip_height.0 > self.max_ancestors_from_best_tip as u64 {
            best_tip_height.0 - self.max_ancestors_from_best_tip as u64
        } else {
            1 // Keep at least the genesis block height
        };

        // Collect heights to prune (those below the min allowed height)
        let heights_to_remove: Vec<Height> = self.tree.keys().filter(|&height| height.0 < min_allowed_height).cloned().collect();

        // Remove all nodes below the minimum allowed height
        for height in heights_to_remove {
            self.tree.remove(&height);
        }

        Ok(())
    }

    pub fn add_node(&mut self, node: Node) -> Result<(), &'static str> {
        if let Some(nodes) = self.tree.get(&node.height) {
            if nodes.contains(&node) {
                return Err("Node already exists");
            }
        }

        if !self.has_parent(&node) {
            return Err("One or more parent nodes do not exist");
        }

        self.tree.entry(node.height.clone()).or_default().push(node);

        Ok(())
    }

    pub fn get_parent(&self, node: &Node) -> Option<&Node> {
        if let Some(parents) = self.tree.get(&Height(node.height.0 - 1)) {
            parents.iter().find(|entry| entry.state_hash == node.previous_state_hash)
        } else {
            None
        }
    }

    pub fn has_parent(&self, node: &Node) -> bool {
        self.get_parent(node).is_some()
    }

    fn sort_entries(entries: &mut [Node]) {
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

    pub fn get_best_tip(&self) -> Result<(Height, Node), &'static str> {
        if let Some((height, nodes)) = self.tree.last_key_value() {
            let mut nodes_cloned = nodes.clone();
            Self::sort_entries(&mut nodes_cloned);
            if let Some(canonical_node) = nodes_cloned.first() {
                Ok((height.clone(), canonical_node.clone()))
            } else {
                Err("Best tip has no nodes")
            }
        } else {
            Err("No best tip found")
        }
    }

    pub fn get_shared_ancestry(&self, node1: &Node, node2: &Node) -> Result<(Vec<Node>, Vec<Node>, Node), &'static str> {
        if node1.height != node2.height {
            return Err("Nodes are at different heights, cannot find shared ancestry");
        }

        let mut ancestry1 = vec![node1.clone()];
        let mut ancestry2 = vec![node2.clone()];
        let mut current_node1 = node1.clone();
        let mut current_node2 = node2.clone();

        loop {
            // If we've reached the same node (i.e., common ancestor), remove it from the ancestry lists and return
            if current_node1.state_hash == current_node2.state_hash {
                ancestry1.pop(); // Remove the common ancestor from ancestry1
                ancestry2.pop(); // Remove the common ancestor from ancestry2
                return Ok((ancestry1, ancestry2, current_node1));
            }

            // Move both nodes one level up to their parents, if possible
            let parent1 = self.get_parent(&current_node1);
            let parent2 = self.get_parent(&current_node2);

            match (parent1, parent2) {
                (Some(p1), Some(p2)) => {
                    ancestry1.push(p1.clone());
                    ancestry2.push(p2.clone());
                    current_node1 = p1.clone();
                    current_node2 = p2.clone();
                }
                _ => return Err("No common ancestor found"),
            }
        }
    }
}

#[cfg(test)]
mod blockchain_tree_tests {
    use super::*;
    use crate::{
        constants::{GENESIS_STATE_HASH, TRANSITION_FRONTIER_DISTANCE},
        stream::payloads::GenesisBlockPayload,
    };

    #[test]
    fn test_add_node_with_parent() {
        let mut tree = BlockchainTree::new(TRANSITION_FRONTIER_DISTANCE);
        let genesis_payload = GenesisBlockPayload::new();
        let node = Node {
            height: Height(genesis_payload.height),
            state_hash: Hash(genesis_payload.state_hash),
            previous_state_hash: Hash(genesis_payload.previous_state_hash),
            last_vrf_output: genesis_payload.last_vrf_output,
            ..Default::default()
        };
        tree.set_root(node).unwrap();

        let node = Node {
            height: Height(2),
            state_hash: Hash("block_2".to_string()),
            previous_state_hash: Hash(GENESIS_STATE_HASH.to_string()),
            last_vrf_output: "vrf_output".to_string(),
            ..Default::default()
        };

        let result = tree.add_node(node.clone());
        assert!(result.is_ok());
        assert!(tree.tree.contains_key(&Height(2)));
        assert_eq!(tree.tree[&Height(2)][0], node);
    }

    #[test]
    fn test_add_node_without_parent_fails() {
        let mut tree = BlockchainTree::new(TRANSITION_FRONTIER_DISTANCE);
        let genesis_payload = GenesisBlockPayload::new();
        let node = Node {
            height: Height(genesis_payload.height),
            state_hash: Hash(genesis_payload.state_hash),
            previous_state_hash: Hash(genesis_payload.previous_state_hash),
            last_vrf_output: genesis_payload.last_vrf_output,
            ..Default::default()
        };
        tree.set_root(node).unwrap();

        let node = Node {
            height: Height(3),
            state_hash: Hash("block_3".to_string()),
            previous_state_hash: Hash("nonexistent_parent".to_string()),
            last_vrf_output: "vrf_output".to_string(),
            ..Default::default()
        };

        let result = tree.add_node(node);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "One or more parent nodes do not exist");
    }

    #[test]
    fn test_add_duplicate_node_fails() {
        let mut tree = BlockchainTree::new(TRANSITION_FRONTIER_DISTANCE);
        let genesis_payload = GenesisBlockPayload::new();
        let node = Node {
            height: Height(genesis_payload.height),
            state_hash: Hash(genesis_payload.state_hash),
            previous_state_hash: Hash(genesis_payload.previous_state_hash),
            last_vrf_output: genesis_payload.last_vrf_output,
            ..Default::default()
        };
        tree.set_root(node).unwrap();

        let node = Node {
            height: Height(2),
            state_hash: Hash("block_2".to_string()),
            previous_state_hash: Hash(GENESIS_STATE_HASH.to_string()),
            last_vrf_output: "vrf_output".to_string(),
            ..Default::default()
        };

        tree.add_node(node.clone()).unwrap();
        let result = tree.add_node(node);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Node already exists");
    }

    #[test]
    fn test_get_best_tip_single_node_at_height() {
        let mut tree = BlockchainTree::new(TRANSITION_FRONTIER_DISTANCE);
        let genesis_payload = GenesisBlockPayload::new();
        let node = Node {
            height: Height(genesis_payload.height),
            state_hash: Hash(genesis_payload.state_hash),
            previous_state_hash: Hash(genesis_payload.previous_state_hash),
            last_vrf_output: genesis_payload.last_vrf_output,
            ..Default::default()
        };
        tree.set_root(node).unwrap();

        let node = Node {
            height: Height(2),
            state_hash: Hash("block_2".to_string()),
            previous_state_hash: Hash(GENESIS_STATE_HASH.to_string()),
            last_vrf_output: "vrf_output".to_string(),
            ..Default::default()
        };

        tree.add_node(node.clone()).unwrap();

        let best_tip = tree.get_best_tip();
        assert!(best_tip.is_ok());
        let (height, best_node) = best_tip.unwrap();
        assert_eq!(height, Height(2));
        assert_eq!(best_node, node);
    }

    #[test]
    fn test_get_best_tip_multiple_nodes_at_height() {
        let mut tree = BlockchainTree::new(TRANSITION_FRONTIER_DISTANCE);
        let genesis_payload = GenesisBlockPayload::new();
        let node = Node {
            height: Height(genesis_payload.height),
            state_hash: Hash(genesis_payload.state_hash),
            previous_state_hash: Hash(genesis_payload.previous_state_hash),
            last_vrf_output: genesis_payload.last_vrf_output,
            ..Default::default()
        };
        tree.set_root(node).unwrap();

        // Add a parent node at height 2
        let parent_node = Node {
            height: Height(2),
            state_hash: Hash("block_2".to_string()),
            previous_state_hash: Hash(GENESIS_STATE_HASH.to_string()),
            last_vrf_output: "parent_vrf".to_string(),
            ..Default::default()
        };
        tree.add_node(parent_node.clone()).unwrap();

        // Nodes at height 3 with the same parent
        let node1 = Node {
            height: Height(3),
            state_hash: Hash("block_3a".to_string()),
            previous_state_hash: Hash("block_2".to_string()),
            last_vrf_output: "c_vrf_output".to_string(),
            ..Default::default()
        };

        let node2 = Node {
            height: Height(3),
            state_hash: Hash("block_3b".to_string()),
            previous_state_hash: Hash("block_2".to_string()),
            last_vrf_output: "d_vrf_output".to_string(),
            ..Default::default()
        };

        let node3 = Node {
            height: Height(3),
            state_hash: Hash("block_3c".to_string()),
            previous_state_hash: Hash("block_2".to_string()),
            last_vrf_output: "b_vrf_output".to_string(),
            ..Default::default()
        };

        let node4 = Node {
            height: Height(3),
            state_hash: Hash("block_3d".to_string()),
            previous_state_hash: Hash("block_2".to_string()),
            last_vrf_output: "a_vrf_output".to_string(),
            ..Default::default()
        };

        // Add nodes to the tree
        tree.add_node(node1.clone()).unwrap();
        tree.add_node(node2.clone()).unwrap();
        tree.add_node(node3.clone()).unwrap();
        tree.add_node(node4.clone()).unwrap();

        let best_tip = tree.get_best_tip();
        assert!(best_tip.is_ok());
        let (height, best_node) = best_tip.unwrap();

        assert_eq!(height, Height(3));
        assert_eq!(best_node, node2);
    }

    #[test]
    fn test_get_best_tip_empty_tree() {
        let empty_tree = BlockchainTree::new(TRANSITION_FRONTIER_DISTANCE);

        let best_tip = empty_tree.get_best_tip();
        assert!(best_tip.is_err());
        assert_eq!(best_tip.unwrap_err(), "No best tip found");
    }

    #[test]
    fn test_get_shared_ancestry() {
        let mut tree = BlockchainTree::new(TRANSITION_FRONTIER_DISTANCE);
        let genesis_payload = GenesisBlockPayload::new();
        let node = Node {
            height: Height(genesis_payload.height),
            state_hash: Hash(genesis_payload.state_hash),
            previous_state_hash: Hash(genesis_payload.previous_state_hash),
            last_vrf_output: genesis_payload.last_vrf_output,
            ..Default::default()
        };
        tree.set_root(node).unwrap();

        let parent_node = Node {
            height: Height(2),
            state_hash: Hash("block_2".to_string()),
            previous_state_hash: Hash(GENESIS_STATE_HASH.to_string()),
            last_vrf_output: "parent_vrf".to_string(),
            ..Default::default()
        };
        tree.add_node(parent_node.clone()).unwrap();

        let node1 = Node {
            height: Height(3),
            state_hash: Hash("block_3a".to_string()),
            previous_state_hash: Hash("block_2".to_string()),
            last_vrf_output: "vrf_output_1".to_string(),
            ..Default::default()
        };

        let node2 = Node {
            height: Height(3),
            state_hash: Hash("block_3b".to_string()),
            previous_state_hash: Hash("block_2".to_string()),
            last_vrf_output: "vrf_output_2".to_string(),
            ..Default::default()
        };

        tree.add_node(node1.clone()).unwrap();
        tree.add_node(node2.clone()).unwrap();

        let result = tree.get_shared_ancestry(&node1, &node2);
        assert!(result.is_ok());

        let (ancestry1, ancestry2, common_ancestor) = result.unwrap();
        assert_eq!(common_ancestor, parent_node);
        assert_eq!(ancestry1, vec![node1.clone()]);
        assert_eq!(ancestry2, vec![node2.clone()]);
    }

    #[test]
    fn test_get_shared_ancestry_complex_case() {
        let mut tree = BlockchainTree::new(TRANSITION_FRONTIER_DISTANCE);
        let genesis_payload = GenesisBlockPayload::new();
        let node = Node {
            height: Height(genesis_payload.height),
            state_hash: Hash(genesis_payload.state_hash),
            previous_state_hash: Hash(genesis_payload.previous_state_hash),
            last_vrf_output: genesis_payload.last_vrf_output,
            ..Default::default()
        };
        tree.set_root(node).unwrap();

        // Define a chain of ancestry for both nodes, with a common ancestor at height 2
        let common_ancestor = Node {
            height: Height(2),
            state_hash: Hash("common_ancestor".to_string()),
            previous_state_hash: Hash(GENESIS_STATE_HASH.to_string()),
            last_vrf_output: "common_vrf".to_string(),
            ..Default::default()
        };
        tree.add_node(common_ancestor.clone()).unwrap();

        // Branch 1: Node chain leading up to node_a
        let branch1_node1 = Node {
            height: Height(3),
            state_hash: Hash("branch1_block3".to_string()),
            previous_state_hash: Hash("common_ancestor".to_string()),
            last_vrf_output: "vrf_b1_3".to_string(),
            ..Default::default()
        };
        let branch1_node2 = Node {
            height: Height(4),
            state_hash: Hash("branch1_block4".to_string()),
            previous_state_hash: Hash("branch1_block3".to_string()),
            last_vrf_output: "vrf_b1_4".to_string(),
            ..Default::default()
        };
        let node_a = Node {
            height: Height(5),
            state_hash: Hash("node_a".to_string()),
            previous_state_hash: Hash("branch1_block4".to_string()),
            last_vrf_output: "vrf_a".to_string(),
            ..Default::default()
        };

        tree.add_node(branch1_node1.clone()).unwrap();
        tree.add_node(branch1_node2.clone()).unwrap();
        tree.add_node(node_a.clone()).unwrap();

        // Branch 2: Node chain leading up to node_b, with some nodes at the same height as branch 1
        let branch2_node1 = Node {
            height: Height(3),
            state_hash: Hash("branch2_block3".to_string()),
            previous_state_hash: Hash("common_ancestor".to_string()),
            last_vrf_output: "vrf_b2_3".to_string(),
            ..Default::default()
        };
        let branch2_node2 = Node {
            height: Height(4),
            state_hash: Hash("branch2_block4".to_string()),
            previous_state_hash: Hash("branch2_block3".to_string()),
            last_vrf_output: "vrf_b2_4".to_string(),
            ..Default::default()
        };
        let node_b = Node {
            height: Height(5),
            state_hash: Hash("branch2_block5".to_string()),
            previous_state_hash: Hash("branch2_block4".to_string()),
            last_vrf_output: "vrf_b2_5".to_string(),
            ..Default::default()
        };

        tree.add_node(branch2_node1.clone()).unwrap();
        tree.add_node(branch2_node2.clone()).unwrap();
        tree.add_node(node_b.clone()).unwrap();

        // Now check the shared ancestry between node_a and node_b
        let result = tree.get_shared_ancestry(&node_a, &node_b);
        assert!(result.is_ok());

        let (ancestry_a, ancestry_b, common_ancestor_found) = result.unwrap();

        // Expected ancestry for node_a should include its chain up to the common ancestor
        let expected_ancestry_a = vec![node_a.clone(), branch1_node2.clone(), branch1_node1.clone()];

        // Expected ancestry for node_b should include its chain up to the common ancestor
        let expected_ancestry_b = vec![node_b.clone(), branch2_node2.clone(), branch2_node1.clone()];

        assert_eq!(ancestry_a, expected_ancestry_a);
        assert_eq!(ancestry_b, expected_ancestry_b);
        assert_eq!(common_ancestor_found, common_ancestor);
    }

    #[test]
    fn test_set_root() {
        let mut tree = BlockchainTree::new(TRANSITION_FRONTIER_DISTANCE);

        // Define a new root node to add at height 2
        let root_payload = GenesisBlockPayload::new();
        let new_root_node = Node {
            height: Height(root_payload.height),
            state_hash: Hash(root_payload.state_hash),
            previous_state_hash: Hash(root_payload.previous_state_hash),
            last_vrf_output: root_payload.last_vrf_output,
            ..Default::default()
        };

        // Call set_root to add the new root node
        let result = tree.set_root(new_root_node.clone());
        assert!(result.is_ok());

        // Verify that the tree now contains the new root node at height 2
        assert!(tree.tree.contains_key(&Height(root_payload.height)));
        assert_eq!(tree.tree[&Height(root_payload.height)][0], new_root_node);
    }
}

#[cfg(test)]
mod blockchain_tree_prune_tests {
    use super::*;

    fn setup_tree_with_height(max_height: u64, max_ancestors: usize) -> BlockchainTree {
        let mut tree = BlockchainTree::new(max_ancestors);

        // Add a series of nodes up to `max_height`
        for height in 1..=max_height {
            let node = Node {
                height: Height(height),
                state_hash: Hash(format!("hash_{}", height)),
                previous_state_hash: if height > 1 {
                    Hash(format!("hash_{}", height - 1))
                } else {
                    Hash("genesis".to_string())
                },
                last_vrf_output: format!("vrf_{}", height),
                ..Default::default()
            };
            tree.set_root(node.clone()).unwrap();
        }

        tree
    }

    #[test]
    fn test_prune_tree_no_pruning_needed() {
        let mut tree = setup_tree_with_height(10, 11);

        // Call prune_tree on a tree that should not be pruned
        tree.prune_tree().unwrap();

        // Ensure no nodes were removed
        assert_eq!(tree.tree.len(), 10);
        assert!(tree.tree.contains_key(&Height(10)));
    }

    #[test]
    fn test_prune_tree_prune_one_node() {
        let mut tree = setup_tree_with_height(10, 8);

        // Call prune_tree on a tree where only one node (height 1) should be pruned
        tree.prune_tree().unwrap();

        // Ensure the node at height 1 was removed
        assert!(!tree.tree.contains_key(&Height(1)));

        // Ensure nodes from height 2 to 10 remain
        for height in 2..=10 {
            assert!(tree.tree.contains_key(&Height(height)));
        }
    }

    #[test]
    fn test_prune_tree_prune_multiple_nodes() {
        let mut tree = setup_tree_with_height(15, 10);

        // Call prune_tree where nodes up to height 5 should be pruned
        tree.prune_tree().unwrap();

        // Ensure nodes from height 1 to 5 were removed
        for height in 1..=4 {
            assert!(!tree.tree.contains_key(&Height(height)));
        }

        // Ensure nodes from height 6 to 15 remain
        for height in 5..=15 {
            assert!(tree.tree.contains_key(&Height(height)));
        }
    }

    #[test]
    fn test_prune_tree_min_allowed_height_equal_to_best_tip() {
        // Set up a tree with only one node and a high max_ancestors_from_best_tip
        let mut tree = BlockchainTree::new(100);
        let genesis_node = Node {
            height: Height(1),
            state_hash: Hash("genesis".to_string()),
            previous_state_hash: Hash("".to_string()),
            last_vrf_output: "".to_string(),
            ..Default::default()
        };
        tree.set_root(genesis_node.clone()).unwrap();

        // Call prune_tree on a tree with only the genesis node
        tree.prune_tree().unwrap();

        // Ensure the genesis node is not pruned
        assert!(tree.tree.contains_key(&Height(1)));
        assert_eq!(tree.tree.len(), 1);
    }

    #[test]
    fn test_prune_tree_no_removal_if_within_max_ancestors() {
        let mut tree = setup_tree_with_height(15, 20);

        // Call prune_tree with a high max_ancestors_from_best_tip
        tree.prune_tree().unwrap();

        // Ensure all nodes remain
        assert_eq!(tree.tree.len(), 15);
        for height in 1..=15 {
            assert!(tree.tree.contains_key(&Height(height)));
        }
    }

    #[test]
    fn test_metadata_str_field() {
        let mut tree = BlockchainTree::new(10);

        // Add a node with metadata
        let node_with_metadata = Node {
            height: Height(1),
            state_hash: Hash("block_4".to_string()),
            previous_state_hash: Hash("block_3".to_string()),
            last_vrf_output: "vrf_output_4".to_string(),
            metadata_str: Some("Important metadata".to_string()),
        };
        tree.set_root(node_with_metadata.clone()).unwrap();

        // Retrieve the node and check the metadata
        let retrieved_node = tree.get_node(Height(1), Hash("block_4".to_string()));
        assert!(retrieved_node.is_some());
        assert_eq!(retrieved_node.unwrap().metadata_str, Some("Important metadata".to_string()));
    }
}
