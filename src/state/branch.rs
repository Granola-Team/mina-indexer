use crate::block::{precomputed::PrecomputedBlock, Block, BlockHash};
use id_tree::{
    InsertBehavior::{AsRoot, UnderNode},
    MoveBehavior::ToRoot,
    Node, NodeId,
    RemoveBehavior::{DropChildren, OrphanChildren},
    Tree,
};
use serde::ser::SerializeStruct;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Clone)]
pub struct Branch {
    pub root: NodeId,
    pub branches: Tree<Block>,
}

pub type Path = Vec<Block>;

#[derive(Clone)]
pub struct Leaf<T> {
    pub block: Block,
    ledger: T,
}

impl Branch {
    pub fn new_genesis(root_hash: BlockHash) -> Self {
        let genesis_block = Block {
            state_hash: root_hash.clone(),
            parent_hash: root_hash,
            height: 0,
            blockchain_length: Some(1),
        };
        let mut branches = Tree::new();

        let root = branches.insert(Node::new(genesis_block), AsRoot).unwrap();

        Self { root, branches }
    }

    pub fn new_non_genesis(root_hash: BlockHash, blockchain_length: Option<u32>) -> Self {
        let root_block = Block {
            state_hash: root_hash.clone(),
            parent_hash: root_hash,
            height: 0,
            blockchain_length,
        };
        let mut branches = Tree::new();
        let root = branches.insert(Node::new(root_block), AsRoot).unwrap();

        Self { root, branches }
    }

    pub fn new_testing(precomputed_block: &PrecomputedBlock) -> Self {
        let root_block = Block::from_precomputed(precomputed_block, 0);
        let mut branches = Tree::new();
        let root = branches.insert(Node::new(root_block), AsRoot).unwrap();

        Self { root, branches }
    }

    // only the genesis block should work here
    pub fn new_rooted(root_precomputed: &PrecomputedBlock) -> Self {
        Branch::new(root_precomputed).unwrap()
    }
}

impl Branch {
    pub fn new(root_precomputed: &PrecomputedBlock) -> anyhow::Result<Self> {
        let root_block = Block::from_precomputed(root_precomputed, 0);
        let mut branches = Tree::new();
        let root = branches.insert(Node::new(root_block), AsRoot)?;

        Ok(Self { root, branches })
    }

    pub fn is_empty(&self) -> bool {
        self.branches.height() == 0
    }

    /// Returns the new node's id in the branch
    pub fn simple_extension(&mut self, block: &PrecomputedBlock) -> Option<(NodeId, Block)> {
        let root_node_id = self
            .branches
            .root_node_id()
            .expect("root_node_id guaranteed by constructor");
        for node_id in self
            .branches
            .traverse_post_order_ids(root_node_id)
            .expect("root_node_id is valid")
        {
            let node = self
                .branches
                .get(&node_id)
                .expect("node_id comes from branches iterator, cannot be invalid");

            // incoming block is a child of node
            let incoming_prev_hash = BlockHash::previous_state_hash(block);
            if incoming_prev_hash == node.data().state_hash {
                let new_block = Block::from_precomputed(block, node.data().height + 1);
                let new_node_id = self
                    .branches
                    .insert(Node::new(new_block.clone()), UnderNode(&node_id))
                    .expect("node_id comes from branches iterator, cannot be invalid");

                return Some((new_node_id, new_block));
            }
        }
        None
    }

    /// Prunes the tree and updates the root
    pub fn prune_transition_frontier(&mut self, k: u32, best_tip: &Block) {
        let mut witness_length = 0;
        let mut new_root_id = None;
        let mut prune_point_id = None;
        let best_tip_id = self.leaf_node_id(best_tip).unwrap();

        for ancestor_id in self.branches.ancestor_ids(&best_tip_id).unwrap().cloned() {
            witness_length += 1;
            if witness_length == k {
                new_root_id = Some(ancestor_id.clone());
            }
            if witness_length == k + 1 {
                prune_point_id = Some(ancestor_id);
            }
        }

        // guaranteed to exist because of the height precondition
        let new_root_id = new_root_id.unwrap();
        let prune_point_id = prune_point_id.unwrap();

        // remove all prune point siblings
        for node_id in self
            .branches
            .get(&prune_point_id)
            .unwrap()
            .children()
            .clone()
        {
            if node_id != new_root_id {
                self.branches
                    .remove_node(node_id.clone(), id_tree::RemoveBehavior::DropChildren)
                    .unwrap();
            }
        }

        // remove parent node + orphan children
        self.branches
            .remove_node(prune_point_id, OrphanChildren)
            .unwrap();

        // remove original root + drop children
        self.branches
            .remove_node(self.branches.root_node_id().unwrap().clone(), DropChildren)
            .unwrap();

        // move prune node to root
        self.branches.move_node(&new_root_id, ToRoot).unwrap();

        // update node heights
        let n = self.branches.get(&new_root_id).unwrap().data().height;
        let node_ids: Vec<NodeId> = self
            .branches
            .traverse_level_order_ids(&new_root_id)
            .unwrap()
            .collect();

        for node_id in node_ids {
            let node = self.branches.get_mut(&node_id).unwrap();
            node.data_mut().height -= n;
        }

        // update root
        self.root = new_root_id.clone();
    }

    /// block is guaranteed to exist in leaves
    fn leaf_node_id(&self, block: &Block) -> Option<NodeId> {
        self.branches
            .traverse_post_order_ids(self.branches.root_node_id()?)
            .unwrap()
            .find(|node_id| {
                block.state_hash == self.branches.get(node_id).unwrap().data().state_hash
            })
    }

    /// Merges two trees:
    /// incoming is placed under junction_id in self
    ///
    /// Returns the id of the best tip in the merged subtree
    pub fn merge_on(&mut self, junction_id: &NodeId, incoming: &mut Branch) -> Option<NodeId> {
        let (merged_tip_id, _) = incoming.best_tip_with_id().unwrap();
        let mut merge_id_map = HashMap::new();

        // associate the incoming tree's root node id with it's new id in the base tree
        let incoming_root_id = incoming
            .branches
            .root_node_id()
            .expect("branch always has root node");
        let mut incoming_root_data = incoming
            .branches
            .get(incoming_root_id)
            .expect("incoming_root_id valid, branch always has root node")
            .data()
            .clone();
        let junction_height = self
            .branches
            .get(junction_id)
            .expect("junction node exists in self")
            .data()
            .height;
        let junction_length = self
            .branches
            .get(junction_id)
            .expect("junction node exists in self")
            .data()
            .blockchain_length
            .unwrap_or(0);

        // adjust the height of the incoming branch's root block
        incoming_root_data.height = junction_height + 1;
        if incoming_root_data.blockchain_length.is_none() {
            incoming_root_data.blockchain_length = Some(junction_length + 1)
        }

        let new_node_id = self
            .branches
            .insert(Node::new(incoming_root_data), UnderNode(junction_id))
            .expect("merge_on called with valid junction_id");

        merge_id_map.insert(incoming_root_id, new_node_id);

        for old_node_id in incoming
            .branches
            .traverse_level_order_ids(incoming_root_id)
            .expect("incoming_root_id guaranteed by root_id() call")
        {
            let under_node_id = merge_id_map
                .get(&old_node_id)
                .expect("guaranteed by call structure");
            let children_ids = incoming
                .branches
                .children_ids(&old_node_id)
                .expect("old_node_id valid");
            let mut merge_id_map_inserts = Vec::new();

            for child_id in children_ids {
                let mut child_node_data = incoming
                    .branches
                    .get(child_id)
                    .expect("child_id valid")
                    .data()
                    .clone();

                child_node_data.height += junction_height + 1;

                if child_node_data.blockchain_length.is_none() {
                    child_node_data.blockchain_length =
                        Some(junction_length + child_node_data.height + 1 - junction_height)
                }

                let new_child_id = self
                    .branches
                    .insert(Node::new(child_node_data), UnderNode(under_node_id))
                    .expect("under_node_id guaranteed by call structure");

                merge_id_map_inserts.push((child_id, new_child_id));
            }

            for (child_id, new_child_id) in merge_id_map_inserts {
                merge_id_map.insert(child_id, new_child_id);
            }
        }

        merge_id_map.get(&merged_tip_id).cloned()
    }

    pub fn new_root(&mut self, precomputed_block: &PrecomputedBlock) {
        let new_block = Block::from_precomputed(precomputed_block, 0);
        let new_root_id = self
            .branches
            .insert(Node::new(new_block), AsRoot)
            .expect("insert as root always succeeds");

        self.root = new_root_id.clone();

        let child_ids: Vec<NodeId> = self
            .branches
            .traverse_level_order_ids(&new_root_id)
            .expect("new_root_id received from .insert() call, is valid")
            .collect();

        for node_id in child_ids {
            let node = self
                .branches
                .get_mut(&node_id)
                .expect("node_id from iterator, cannot be invalid");

            if node_id != new_root_id {
                let mut block = node.data().clone();
                block.height += 1;
                node.replace_data(block);
            }
        }
    }

    pub fn root_block(&self) -> &Block {
        self.branches.get(&self.root).unwrap().data()
    }

    pub fn top_leaves_with_id(&self) -> Vec<(NodeId, Block)> {
        let mut top_leaves = vec![];

        for node_id in self
            .branches
            .traverse_post_order_ids(self.branches.root_node_id().unwrap())
            .unwrap()
        {
            let node = self.branches.get(&node_id).unwrap();
            if node.data().height + 1 == self.height() {
                top_leaves.push((node_id.clone(), node.data().clone()));
            }
        }

        top_leaves
    }

    pub fn top_leaves(&self) -> Vec<Block> {
        self.top_leaves_with_id()
            .iter()
            .map(|(_, block)| block)
            .cloned()
            .collect()
    }

    pub fn leaves(&self) -> Vec<Block> {
        let mut leaves = vec![];

        for node in self
            .branches
            .traverse_level_order(self.branches.root_node_id().unwrap())
            .unwrap()
        {
            if node.data().height + 1 == self.height() {
                leaves.push(node.data().clone());
            }
        }

        leaves
    }

    // Always returns some for a non-empty tree
    pub fn best_tip(&self) -> Option<Block> {
        self.best_tip_with_id().map(|(_, x)| x)
    }

    // Always returns some for a non-empty tree
    pub fn best_tip_with_id(&self) -> Option<(NodeId, Block)> {
        let mut leaves = self.top_leaves_with_id();
        leaves.sort_by(|(_, x), (_, y)| x.cmp(y).reverse());
        leaves.first().cloned()
    }

    pub fn longest_chain(&self) -> Vec<BlockHash> {
        let mut longest_chain = Vec::new();
        if let Some((node_id, _)) = self.best_tip_with_id() {
            // push the leaf itself
            longest_chain.push(
                self.branches
                    .get(&node_id)
                    .unwrap()
                    .data()
                    .state_hash
                    .clone(),
            );

            // push the leaf's ancestors
            for node in self.branches.ancestors(&node_id).expect("node_id is valid") {
                longest_chain.push(node.data().state_hash.clone());
            }
        }

        longest_chain
    }

    pub fn len(&self) -> u32 {
        let mut size = 0;
        if let Some(root) = self.branches.root_node_id() {
            for _ in self.branches.traverse_level_order_ids(root).unwrap() {
                size += 1;
            }
        }
        size
    }

    pub fn height(&self) -> u32 {
        self.branches.height() as u32
    }

    pub fn mem(&self, state_hash: &BlockHash) -> bool {
        for node in self
            .branches
            .traverse_post_order(self.branches.root_node_id().unwrap())
            .unwrap()
        {
            if &node.data().state_hash == state_hash {
                return true;
            }
        }
        false
    }
}

impl<T> Leaf<T> {
    pub fn new(data: Block, ledger: T) -> Self {
        Self {
            block: data,
            ledger,
        }
    }

    pub fn get_ledger(&self) -> &T {
        &self.ledger
    }
}

impl<T> Serialize for Leaf<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Leaf", 2)?;
        state.serialize_field("block", &self.block)?;
        state.serialize_field("ledger", &self.ledger)?;
        state.end()
    }
}

// only display the underlying tree
impl std::fmt::Debug for Branch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut tree = String::new();
        self.branches.write_formatted(&mut tree)?;
        write!(f, "{tree}")
    }
}