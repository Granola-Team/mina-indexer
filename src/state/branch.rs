use std::collections::HashMap;

use id_tree::{InsertBehavior, Node, NodeId, Tree};

use crate::block::Block;

#[derive(Debug, Clone)]
pub struct Branch {
    pub root: Block,
    pub branches: Tree<Leaf>,
    pub leaves: Leaves,
}

pub type Path = Vec<Block>;

pub type Leaves = HashMap<NodeId, Leaf>;

#[derive(Debug, Clone)]
pub struct Leaf {
    data: Block,
    depth: usize,
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
    pub fn new(root: Block) -> Result<Self, anyhow::Error> {
        let mut branches = Tree::new();
        let root_id = branches.insert(
            Node::new(Leaf::new(root.clone(), 0)),
            InsertBehavior::AsRoot,
        )?;

        let mut leaves = HashMap::new();
        leaves.insert(root_id, Leaf::new(root.clone(), 0));
        Ok(Self {
            root,
            branches,
            leaves,
        })
    }

    pub fn simple_extension(&mut self, block: Block) {
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

            if block.parent_hash == node.data().data.state_hash {
                let new_leaf = Leaf::new(block, node.data().depth + 1);
                let new_node_id = self
                    .branches
                    .insert(
                        Node::new(new_leaf.clone()),
                        InsertBehavior::UnderNode(&node_id),
                    )
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

impl std::hash::Hash for Branch {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.root.hash(state);
    }
}
impl PartialEq for Branch {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root && self.branches == other.branches && self.leaves == other.leaves
    }
}
impl Eq for Branch {}

impl Leaf {
    pub fn new(data: Block, depth: usize) -> Self {
        Self { data, depth }
    }
}
impl PartialEq for Leaf {
    fn eq(&self, other: &Self) -> bool {
        self.depth == other.depth
    }
}
impl Eq for Leaf {}
impl PartialOrd for Leaf {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.depth.partial_cmp(&other.depth)
    }
}
impl Ord for Leaf {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.depth.cmp(&other.depth)
    }
}
