use std::{collections::HashMap, rc::Rc};

use crate::Block;

#[derive(Debug)]
pub struct SubchainContext {
    current_blocks: HashMap<String, Rc<Block>>,
    current_chains: HashMap<String, SubchainTree>,
}

#[derive(Hash, Clone, Debug, PartialEq)]
pub enum SubchainTree{
    Leaf(Rc<Block>),
    Root(Rc<Block>, Vec<SubchainTree>),
}

impl SubchainTree {
    pub fn add_block(&self, block: Rc<Block>) -> Self {
        match self {
            SubchainTree::Leaf(root) => {
                if root.state_hash == block.protocol_state.previous_state_hash {
                    SubchainTree::Root(root.clone(), vec![SubchainTree::Leaf(block)])
                } else {
                    SubchainTree::Leaf(root.clone())
                }
            },
            SubchainTree::Root(root, leaves) => {
                if root.state_hash == block.protocol_state.previous_state_hash {
                    let mut leaves = leaves.clone();
                    leaves.push(SubchainTree::Leaf(block));
                    SubchainTree::Root(root.clone(), leaves)
                } else {
                    let mut leaves = leaves.clone();
                    for leaf in leaves.iter_mut() {
                        *leaf = leaf.add_block(block.clone());
                    }
                    SubchainTree::Root(root.clone(), leaves)
                }
            },
        }
    }

    pub fn longest_subchain(&self) -> Vec<Rc<Block>> {
        let mut chain = Vec::new();
        match self {
            SubchainTree::Leaf(block) => vec![block.clone()],
            SubchainTree::Root(block, leaves) => {
                chain.push(block.clone());
                let mut biggest_subchain = Vec::new();
                for leaf in leaves.iter() {
                    let next_chain = leaf.longest_subchain();
                    if next_chain.len() > biggest_subchain.len() {
                        biggest_subchain = next_chain;
                    }
                }
                chain.append(&mut biggest_subchain);
                chain
            },
        }
    }
}

impl SubchainContext {
    pub fn new() -> Self {
        SubchainContext {
            current_blocks: HashMap::new(),
            current_chains: HashMap::new(),
        }
    }

    pub fn recv_block(&mut self, block: Block) {
        let state_hash = block.state_hash.clone();
        let block_rc = Rc::new(block);
        self.current_blocks.insert(state_hash.clone(), block_rc.clone());

        let mut added = false;
        for (_base_hash, tree) in self.current_chains.iter_mut() {
            let new = tree.add_block(block_rc.clone());
            if &new != tree {
                added = true;
                *tree = new;
            }
        }

        if !added {
            self.current_chains.insert(state_hash, SubchainTree::Leaf(block_rc.clone()));
        }
    }

    pub fn longest_chain(&self) -> Vec<String> {
        let mut longest_chain = Vec::new();
        for (_base_hash, tree) in self.current_chains.iter() {
            let tree_chain = tree.longest_subchain();
            if tree_chain.len() >= longest_chain.len() {
                longest_chain = tree_chain;
            }
        }

        longest_chain.iter()
            .map(|block| block.state_hash.clone())
            .collect()
    }
}