use std::collections::HashMap;

use id_tree::{InsertBehavior, Node, NodeId, Tree};

use crate::block::Block;

#[derive(Debug)]
pub struct Branch {
    pub root: Block,
    pub branches: Tree<Block>,
    pub leaves: Leaves,
}

pub type Path = Vec<Block>;

pub type Leaves = HashMap<NodeId, Leaf<Block>>;

#[derive(Debug, Clone)]
pub struct Leaf<T> {
    data: T,
    depth: usize,
} // leaf tracks depth in tree, gives longest path easily

impl Branch {
    pub fn new(root: Block) -> Result<Self, anyhow::Error> {
        let mut branches = Tree::new();
        let root_id = branches.insert(Node::new(root.clone()), InsertBehavior::AsRoot)?;

        let mut leaves = HashMap::new();
        leaves.insert(root_id, Leaf::new(root.clone(), 0));
        Ok(Self {
            root,
            branches,
            leaves,
        })
    }

    pub fn try_add_block(&mut self, block: &Block) -> Result<(), anyhow::Error> {
        let mut to_remove = None;
        for (node_id, Leaf { data, depth }) in self.leaves.iter() {
            if block.parent_hash == data.state_hash {
                let child_id = self
                    .branches
                    .insert(Node::new(block.clone()), InsertBehavior::UnderNode(node_id))?;

                let parent_id = node_id.clone();
                let leaf = Leaf::new(block.clone(), *depth + 1);
                to_remove = Some((parent_id, (child_id, leaf)));
                break; // block will only have one parent
            }
        }

        if let Some((parent_id, (child_id, leaf))) = to_remove {
            self.leaves.remove(&parent_id);
            self.leaves.insert(child_id, leaf);
        }

        Ok(())
    }

    pub fn longest_path(&self) -> Path {
        let mut longest_path = Vec::new();
        if let Some((node_id, leaf)) = self.leaves.iter().max() {
            let mut node = self.branches.get(node_id).unwrap();
            longest_path.push(leaf.data.clone());
            while let Some(parent_id) = node.parent() {
                node = self.branches.get(parent_id).unwrap();
                longest_path.push(node.data().clone());
            }
        }
        longest_path.reverse();
        longest_path
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

impl<T> Leaf<T> {
    pub fn new(data: T, depth: usize) -> Self {
        Self { data, depth }
    }
}
impl<T> PartialEq for Leaf<T> {
    fn eq(&self, other: &Self) -> bool {
        self.depth == other.depth
    }
}
impl<T> Eq for Leaf<T> {}
impl<T> PartialOrd for Leaf<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.depth.partial_cmp(&other.depth)
    }
}
impl<T> Ord for Leaf<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.depth.cmp(&other.depth)
    }
}
