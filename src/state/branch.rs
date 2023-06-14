use crate::{
    block::{precomputed::PrecomputedBlock, Block, BlockHash},
    state::ledger::{
        genesis::GenesisLedger,
        ExtendWithLedgerDiff,
        {diff::LedgerDiff, Ledger},
    },
};
use id_tree::{
    InsertBehavior::{AsRoot, UnderNode},
    MoveBehavior::ToRoot,
    Node, NodeId,
    RemoveBehavior::{DropChildren, OrphanChildren},
    Tree,
};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::marker::PhantomData;

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
    pub fn new_genesis(root_hash: BlockHash, genesis_ledger: Option<GenesisLedger>) -> Self {
        let genesis_block = Block {
            state_hash: root_hash.clone(),
            parent_hash: root_hash,
            height: 0,
            blockchain_length: Some(1),
        };
        let genesis_ledger = match genesis_ledger {
            Some(genesis_ledger) => genesis_ledger.into(),
            None => Ledger::default(),
        };

        let mut branches = Tree::new();
        let root_leaf = Leaf::new(genesis_block.clone(), genesis_ledger);
        let root_id = branches
            .insert(Node::new(root_leaf.clone()), AsRoot)
            .unwrap();

        let mut leaves = HashMap::new();
        leaves.insert(root_id, root_leaf);
        Self {
            root: genesis_block,
            branches,
            leaves,
        }
    }

    pub fn new_testing(precomputed_block: &PrecomputedBlock, root_ledger: Option<Ledger>) -> Self {
        let root_block = Block::from_precomputed(precomputed_block, 0);
        let root_ledger = match root_ledger {
            Some(root_ledger) => root_ledger,
            None => Ledger::default(),
        };

        let mut branches = Tree::new();
        let root_leaf = Leaf::new(root_block.clone(), root_ledger);
        let root_id = branches
            .insert(Node::new(root_leaf.clone()), AsRoot)
            .unwrap();

        let mut leaves = HashMap::new();
        leaves.insert(root_id, root_leaf);
        Self {
            root: root_block,
            branches,
            leaves,
        }
    }

    // only the genesis block should work here
    pub fn new_rooted(root_precomputed: &PrecomputedBlock) -> Self {
        let new_ledger = Ledger::from_diff(LedgerDiff::from_precomputed_block(root_precomputed));
        Branch::new(root_precomputed, new_ledger).unwrap()
    }
}

impl Branch<LedgerDiff> {
    pub fn new_rooted(root_precomputed: &PrecomputedBlock) -> Self {
        let diff = LedgerDiff::from_precomputed_block(root_precomputed);
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
            .traverse_post_order_ids(root_node_id)
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
                    .extend_with_diff(LedgerDiff::from_precomputed_block(block));
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

    /// Prunes the tree and updates the root and leaves
    pub fn prune_transition_frontier(&mut self, k: u32, best_tip: &Block) {
        let mut witness_length = 0;
        let mut new_root_id = None;
        let mut prune_point_id = None;
        let best_tip_id = self.leaf_node_id(best_tip).unwrap();

        for ancestor_id in self.branches.ancestor_ids(best_tip_id).unwrap().cloned() {
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
        let n = self.branches.get(&new_root_id).unwrap().data().block.height;
        let node_ids: Vec<NodeId> = self
            .branches
            .traverse_level_order_ids(&new_root_id)
            .unwrap()
            .collect();
        for node_id in node_ids {
            let node = self.branches.get_mut(&node_id).unwrap();
            node.data_mut().block.height -= n;
        }

        // update root
        self.root = self
            .branches
            .get(&new_root_id)
            .unwrap()
            .data()
            .block
            .clone();

        // update leaves
        let mut to_remove = vec![];
        for leaf_id in self.leaves.keys().cloned() {
            if self.branches.get(&leaf_id).is_err() {
                to_remove.push(leaf_id);
            }
        }
        for leaf_id in to_remove {
            self.leaves.remove(&leaf_id);
        }
    }

    /// block is guaranteed to exist in leaves
    fn leaf_node_id(&self, block: &Block) -> Option<&NodeId> {
        for (node_id, leaf) in self.leaves.iter() {
            if leaf.block.state_hash == block.state_hash {
                return Some(node_id);
            }
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
            ExtendWithLedgerDiff::from_diff(LedgerDiff::from_precomputed_block(precomputed_block));
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

    pub fn longest_chain(&self) -> Vec<BlockHash> {
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
            longest_chain.push(
                self.branches
                    .get(node_id)
                    .unwrap()
                    .data()
                    .block
                    .state_hash
                    .clone(),
            );

            // push the leaf's ancestors
            for node in self.branches.ancestors(node_id).expect("node_id is valid") {
                longest_chain.push(node.data().block.state_hash.clone());
            }
        }

        longest_chain
    }

    pub fn len(&self) -> usize {
        let mut size = 0;
        if let Some(root) = self.branches.root_node_id() {
            for _ in self.branches.traverse_level_order_ids(root).unwrap() {
                size += 1;
            }
        }
        size
    }

    pub fn height(&self) -> usize {
        self.branches.height()
    }

    pub fn mem(&self, state_hash: &BlockHash) -> bool {
        for node in self
            .branches
            .traverse_post_order(self.branches.root_node_id().unwrap())
            .unwrap()
        {
            if &node.data().block.state_hash == state_hash {
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

use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use std::fmt;
impl<'de, T> Deserialize<'de> for Leaf<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Block,
            Ledger,
        }

        struct LeafVisitor<T>(PhantomData<T>);

        impl<'de, T> Visitor<'de> for LeafVisitor<T>
        where
            T: Deserialize<'de>,
        {
            type Value = Leaf<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Leaf")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Leaf<T>, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let block = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let ledger = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(Leaf::new(block, ledger))
            }

            fn visit_map<V>(self, mut map: V) -> Result<Leaf<T>, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut block = None;
                let mut ledger = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Block => {
                            if block.is_some() {
                                return Err(de::Error::duplicate_field("block"));
                            }
                            block = Some(map.next_value()?);
                        }
                        Field::Ledger => {
                            if ledger.is_some() {
                                return Err(de::Error::duplicate_field("ledger"));
                            }
                            ledger = Some(map.next_value()?);
                        }
                    }
                }

                let block = block.ok_or_else(|| de::Error::missing_field("block"))?;
                let ledger = ledger.ok_or_else(|| de::Error::missing_field("ledger"))?;
                Ok(Leaf::new(block, ledger))
            }
        }

        const FIELDS: &[&str] = &["block", "ledger"];
        deserializer.deserialize_struct("Leaf", FIELDS, LeafVisitor(PhantomData))
    }
}

// only display the underlying tree
impl<T> std::fmt::Debug for Branch<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut tree = String::new();
        self.branches.write_formatted(&mut tree)?;
        write!(f, "{tree}")
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

impl<T> PartialEq for Leaf<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.block == other.block && self.ledger == other.ledger
    }
}

#[cfg(test)]
mod leaf_ser_de_tests {
    use std::path::Path;

    use crate::{
        block::{parse_file, Block},
        state::ledger::{diff::LedgerDiff, Ledger},
    };

    use super::Leaf;
    use serde_test::assert_tokens;

    const PRECOMPUTED_BLOCK_PATH: &'static str =
        "./tests/data/beautified_logs/mainnet-175222-3NKn7ZtT6Axw3hK3HpyUGRxmirkuUhtR4cYzWFk75NCgmjCcqPby.json";

    #[tokio::test]
    async fn test_ser_de_ledger() {
        let precomputed_block = parse_file(Path::new(PRECOMPUTED_BLOCK_PATH)).await.unwrap();
        let block = Block::from_precomputed(&precomputed_block, 0);
        let ledger = Ledger::new();
        let leaf = Leaf::new(block, ledger);

        use serde_test::Token::*;
        assert_tokens(
            &leaf,
            &[
                Struct {
                    name: "Leaf",
                    len: 2,
                },
                Str("block"),
                Struct {
                    name: "Block",
                    len: 4,
                },
                Str("parent_hash"),
                NewtypeStruct { name: "BlockHash" },
                Str("3NLxYEKMwRHaRaTmTEs48h2Ds1d45z5DrW3uLDtbMSQ4GCHt4zcc"),
                Str("state_hash"),
                NewtypeStruct { name: "BlockHash" },
                Str("3NKn7ZtT6Axw3hK3HpyUGRxmirkuUhtR4cYzWFk75NCgmjCcqPby"),
                Str("height"),
                U32(0),
                Str("blockchain_length"),
                Some,
                U32(175222),
                StructEnd,
                Str("ledger"),
                Struct {
                    name: "Ledger",
                    len: 1,
                },
                Str("accounts"),
                Map {
                    len: Option::Some(0),
                },
                MapEnd,
                StructEnd,
                StructEnd,
            ],
        )
    }

    #[tokio::test]
    async fn test_ser_de_ledger_diff() {
        let precomputed_block = parse_file(Path::new(PRECOMPUTED_BLOCK_PATH)).await.unwrap();
        let block = Block::from_precomputed(&precomputed_block, 0);
        let ledger = LedgerDiff {
            public_keys_seen: Vec::new(),
            account_diffs: Vec::new(),
        };
        let leaf = Leaf::new(block, ledger);

        use serde_test::Token::*;
        assert_tokens(
            &leaf,
            &[
                Struct {
                    name: "Leaf",
                    len: 2,
                },
                Str("block"),
                Struct {
                    name: "Block",
                    len: 4,
                },
                Str("parent_hash"),
                NewtypeStruct { name: "BlockHash" },
                Str("3NLxYEKMwRHaRaTmTEs48h2Ds1d45z5DrW3uLDtbMSQ4GCHt4zcc"),
                Str("state_hash"),
                NewtypeStruct { name: "BlockHash" },
                Str("3NKn7ZtT6Axw3hK3HpyUGRxmirkuUhtR4cYzWFk75NCgmjCcqPby"),
                Str("height"),
                U32(0),
                Str("blockchain_length"),
                Some,
                U32(175222),
                StructEnd,
                Str("ledger"),
                Struct {
                    name: "LedgerDiff",
                    len: 2,
                },
                Str("public_keys_seen"),
                Seq {
                    len: Option::Some(0),
                },
                SeqEnd,
                Str("account_diffs"),
                Seq {
                    len: Option::Some(0),
                },
                SeqEnd,
                StructEnd,
                StructEnd,
            ],
        );
    }
}
