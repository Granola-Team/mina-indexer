use std::collections::HashMap;

use id_tree::{InsertBehavior::*, Node, NodeId, Tree};

use crate::block::{precomputed::PrecomputedBlock, Block, BlockHash};

use crate::state::ledger::ExtendWithLedgerDiff;

use super::ledger::{diff::LedgerDiff, Ledger};

#[derive(Clone)]
pub struct Branch<T> {
    pub root: Block,
    pub branches: Tree<Leaf<T>>,
    pub leaves: Leaves<T>,
}

pub type Path = Vec<Block>;

pub type Leaves<T> = HashMap<NodeId, Leaf<T>>;

#[derive(Clone)]
pub struct Leaf<T> {
    pub block: Block,
    ledger: T, // add ledger diff here on dangling, ledger on rooted
} // leaf tracks depth in tree, gives longest path easily

pub struct BranchUpdate<T> {
    base_node_id: NodeId,
    new_node_id: NodeId,
    new_leaf: Leaf<T>,
}

impl Branch<Ledger> {
    // only the genesis block should work here
    pub fn new_rooted(root_precomputed: &PrecomputedBlock) -> Self {
        let new_ledger = Ledger::from_diff(LedgerDiff::fom_precomputed_block(root_precomputed));
        Branch::new(root_precomputed, new_ledger).unwrap()
    }
}

impl Branch<LedgerDiff> {
    pub fn new_rooted(root_precomputed: &PrecomputedBlock) -> Self {
        let diff = LedgerDiff::fom_precomputed_block(root_precomputed);
        Branch::new(root_precomputed, diff).unwrap()
    }
}

impl<T> Branch<T>
where
    T: ExtendWithLedgerDiff + Clone,
{
    pub fn new(root_precomputed: &PrecomputedBlock, ledger: T) -> anyhow::Result<Self> {
        let root = Block::from_precomputed(root_precomputed, 0);
        let mut branches = Tree::new();
        let root_leaf = Leaf::new(root.clone(), ledger);
        let root_id = branches.insert(Node::new(root_leaf.clone()), AsRoot)?;

        let mut leaves = HashMap::new();
        leaves.insert(root_id, root_leaf);
        Ok(Self {
            root,
            branches,
            leaves,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.branches.height() == 0
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

            // incoming block is a child of node
            let incoming_prev_hash = BlockHash::previous_state_hash(block);
            if incoming_prev_hash == node.data().block.state_hash {
                let new_block = Block::from_precomputed(block, node.data().block.height + 1);
                let new_ledger = node
                    .data()
                    .ledger
                    .clone()
                    .extend_with_diff(LedgerDiff::fom_precomputed_block(block));
                let new_leaf = Leaf::new(new_block, new_ledger);
                let new_node_id = self
                    .branches
                    .insert(Node::new(new_leaf.clone()), UnderNode(&node_id))
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

    pub fn merge_on(&mut self, junction_id: &NodeId, incoming: &mut Branch<LedgerDiff>) {
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
            .block
            .height;
        let junction_length = self
            .branches
            .get(junction_id)
            .expect("junction node exists in self")
            .data()
            .block
            .blockchain_length
            .unwrap_or(0);
        // adjust the height of the incoming branch's root block
        incoming_root_data.block.height = junction_height + 1;
        if incoming_root_data.block.blockchain_length.is_none() {
            incoming_root_data.block.blockchain_length = Some(junction_length + 1)
        }
        let incoming_branch_root = Leaf::new(
            incoming_root_data.block.clone(),
            ExtendWithLedgerDiff::from_diff(incoming_root_data.ledger),
        );
        let new_node_id = self
            .branches
            .insert(Node::new(incoming_branch_root), UnderNode(junction_id))
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
                child_node_data.block.height += junction_height + 1;
                if child_node_data.block.blockchain_length.is_none() {
                    child_node_data.block.blockchain_length =
                        Some(junction_length + child_node_data.block.height + 1 - junction_height)
                }
                let new_child_node_data = Leaf::new(
                    child_node_data.block,
                    ExtendWithLedgerDiff::from_diff(child_node_data.ledger),
                );
                let new_child_id = self
                    .branches
                    .insert(Node::new(new_child_node_data), UnderNode(under_node_id))
                    .expect("under_node_id guaranteed by call structure");
                merge_id_map_inserts.push((child_id, new_child_id));
            }
            for (child_id, new_child_id) in merge_id_map_inserts {
                merge_id_map.insert(child_id, new_child_id);
            }
        }

        self.leaves.remove(junction_id);
        for (node_id, leaf) in incoming.leaves.iter() {
            if let Some(new_node_id) = merge_id_map.get(node_id) {
                let mut block = incoming.leaves.get(node_id).unwrap().block.clone();
                block.height += junction_height + 1;
                if block.blockchain_length.is_none() {
                    block.blockchain_length =
                        Some(junction_length + block.height + 1 - junction_height)
                }
                let new_leaf =
                    Leaf::new(block, ExtendWithLedgerDiff::from_diff(leaf.ledger.clone()));
                self.leaves.insert(new_node_id.clone(), new_leaf);
            }
        }
    }

    pub fn new_root(&mut self, precomputed_block: &PrecomputedBlock) -> NodeId {
        let new_block = Block::from_precomputed(precomputed_block, 0);
        let new_ledger =
            ExtendWithLedgerDiff::from_diff(LedgerDiff::fom_precomputed_block(precomputed_block));
        let new_leaf = Leaf::new(new_block.clone(), new_ledger);
        let new_root_id = self
            .branches
            .insert(Node::new(new_leaf), AsRoot)
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
                block.block.height += 1;
                node.replace_data(block);
            }
        }
        for (_, leaf) in self.leaves.iter_mut() {
            leaf.block.height += 1;
        }
        new_root_id
    }

    pub fn longest_chain(&self) -> Vec<Leaf<T>> {
        let mut longest_chain = Vec::new();
        let mut highest_leaf = None;
        for (node_id, leaf) in self.leaves.iter() {
            match highest_leaf {
                Some((_node_id, height)) => {
                    if leaf.block.height > height {
                        highest_leaf = Some((node_id, leaf.block.height));
                    }
                }
                None => highest_leaf = Some((node_id, leaf.block.height)),
            }
        }

        if let Some((node_id, _height)) = highest_leaf {
            // push the leaf itself
            longest_chain.push(self.branches.get(node_id).unwrap().data().clone());

            // push the leaf's ancestors
            for node in self.branches.ancestors(node_id).expect("node_id is valid") {
                longest_chain.push(node.data().clone());
            }
        }

        longest_chain
    }
}

impl<T> Leaf<T> {
    pub fn new(data: Block, ledger: T) -> Self {
        Self {
            block: data,
            ledger,
        }
    }
}

// only display the underlying tree
impl<T> std::fmt::Debug for Branch<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut tree = String::new();
        self.branches.write_formatted(&mut tree).unwrap();
        writeln!(f, "Branch {{").unwrap();
        write!(f, "{tree}}}")
    }
}

// only display the underlying block
impl<T> std::fmt::Debug for Leaf<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.block)
    }
}
