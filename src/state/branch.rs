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

// TODO: switch to using Node { block, depth } and Leaf { node, ledger[diff] } or an enum?

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

    pub fn simple_extension(&mut self, block: &PrecomputedBlock) {
        let mut branch_update = None;
        let root_node_id = self
            .branches
            .root_node_id()
            .expect("branch always has a root node");
        for node_id in self
            .branches
            .traverse_level_order_ids(root_node_id)
            .expect("no node id error")
        {
            let node = self
                .branches
                .get(&node_id)
                .expect("node_id received from iterator, is valid");

            if BlockHash::from_bytes(block.protocol_state.previous_state_hash.clone().inner())
                == node.data().state_hash
            {
                let new_block = Block::from_precomputed(block, node.data().slot + 1);
                let new_leaf = Leaf::new(new_block.clone());
                let new_node_id = self
                    .branches
                    .insert(Node::new(new_block), InsertBehavior::UnderNode(&node_id))
                    .expect("node_id received from iterator, is valid");

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
            self.leaves.insert(new_node_id, new_leaf);
            if self.leaves.contains_key(&base_node_id) {
                self.leaves.remove(&base_node_id);
            }
        }
    }
}

impl Leaf {
    pub fn new(data: Block) -> Self {
        Self { block: data }
    }
}
