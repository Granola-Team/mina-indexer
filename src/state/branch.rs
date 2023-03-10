use std::collections::HashMap;

use id_tree::{InsertBehavior, Node, NodeId, Tree};

use crate::block::{precomputed::PrecomputedBlock, Block, BlockHash};

#[derive(Debug, Clone)]
pub struct Branch {
    pub root: Block,
    pub branches: Tree<Block>,
    pub leaves: Leaves,
}

pub type Path = Vec<Block>;

pub type Leaves = HashMap<NodeId, Leaf>;

#[derive(Debug, Clone)]
pub struct Leaf {
    pub block: Block,
    //ledger: T
    // add ledger diff here on dangling, ledger on rooted
} // leaf tracks depth in tree, gives longest path easily

// pub type RootedLeaf = Leaf<Ledger>;
// pub type DanglingLeaf = Leaf<LedgerDiff>;

pub struct BranchUpdate {
    base_node_id: NodeId,
    new_node_id: NodeId,
    new_leaf: Leaf,
}

impl Branch {
    pub fn new(root_precomputed: &PrecomputedBlock) -> Result<Self, anyhow::Error> {
        let root = Block::from_precomputed(root_precomputed, 0);
        let mut branches = Tree::new();
        let root_id = branches.insert(Node::new(root.clone()), InsertBehavior::AsRoot)?;

        let mut leaves = HashMap::new();
        leaves.insert(root_id, Leaf::new(root.clone()));
        Ok(Self {
            root,
            branches,
            leaves,
        })
    }

    pub fn simple_extension(&mut self, block: &PrecomputedBlock) -> Option<NodeId> {
        let mut branch_update = None;
        let root_node_id = self
            .branches
            .root_node_id()
            .expect("root_node_id guaranteed by constructor");
        for node_id in self
            .branches
            .traverse_level_order_ids(root_node_id)
            .expect("root_node_id is valid")
        {
            let node = self
                .branches
                .get(&node_id)
                .expect("node_id comes from branches iterator, cannot be invalid");

            if BlockHash::from_bytes(block.protocol_state.previous_state_hash.clone().inner())
                == node.data().state_hash
            {
                let new_block = Block::from_precomputed(block, node.data().height + 1);
                let new_leaf = Leaf::new(new_block.clone());
                let new_node_id = self
                    .branches
                    .insert(Node::new(new_block), InsertBehavior::UnderNode(&node_id))
                    .expect("node_id comes from branches iterator, cannot be invalid");

                branch_update = Some(BranchUpdate {
                    base_node_id: node_id,
                    new_node_id,
                    new_leaf,
                });
                break;
            }
        }

        if let Some(BranchUpdate {
            base_node_id,
            new_node_id,
            new_leaf,
        }) = branch_update
        {
            self.leaves.insert(new_node_id.clone(), new_leaf);
            if self.leaves.contains_key(&base_node_id) {
                self.leaves.remove(&base_node_id);
            }
            return Some(new_node_id);
        }
        None
    }

    pub fn merge_on(&mut self, junction_id: &NodeId, other: &mut Self) {
        let mut merge_id_map = HashMap::new();
        // associate the incoming tree's root node id with it's new id in the base tree
        let incoming_root_id = other
            .branches
            .root_node_id()
            .expect("branch always has root node");
        let incoming_root_data = other
            .branches
            .get(incoming_root_id)
            .expect("incoming_root_id valid, branch always has root node")
            .data();
        let new_node_id = self
            .branches
            .insert(
                Node::new(incoming_root_data.clone()),
                InsertBehavior::UnderNode(junction_id),
            )
            .expect("merge_on called with valid junction_id");
        merge_id_map.insert(incoming_root_id, new_node_id);
        for old_node_id in other
            .branches
            .traverse_level_order_ids(incoming_root_id)
            .expect("incoming_root_id guaranteed by root_id() call")
        {
            let under_node_id = merge_id_map
                .get(&old_node_id)
                .expect("guaranteed by call structure");
            let children_ids = other
                .branches
                .children_ids(&old_node_id)
                .expect("old_node_id valid");
            let mut merge_id_map_inserts = Vec::new();
            for child_id in children_ids {
                let child_node_data = other.branches.get(child_id).expect("child_id valid").data();
                let new_child_id = self
                    .branches
                    .insert(
                        Node::new(child_node_data.clone()),
                        InsertBehavior::UnderNode(under_node_id),
                    )
                    .expect("under_node_id guaranteed by call structure");
                merge_id_map_inserts.push((child_id, new_child_id));
            }
            for (child_id, new_child_id) in merge_id_map_inserts {
                merge_id_map.insert(child_id, new_child_id);
            }
        }

        if let Some(_leaf) = self.leaves.get(junction_id) {
            self.leaves.remove(junction_id);
        }
        for (node_id, leaf) in other.leaves.iter() {
            if let Some(new_node_id) = merge_id_map.get(node_id) {
                self.leaves.insert(new_node_id.clone(), leaf.clone());
            }
        }
    }

    pub fn new_root(&mut self, precomputed_block: &PrecomputedBlock) {
        let new_block = Block::from_precomputed(precomputed_block, 0);
        let new_root_id = self
            .branches
            .insert(Node::new(new_block.clone()), InsertBehavior::AsRoot)
            .expect("insert as root always succeeds");
        self.root = new_block;
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
}

impl Leaf {
    pub fn new(data: Block) -> Self {
        Self { block: data }
    }
}
