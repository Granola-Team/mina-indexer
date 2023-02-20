use std::collections::HashSet;

use super::block::Block;

#[derive(Debug, PartialEq, Eq)]
pub struct Branches {
    pub branches: HashSet<Branch>,
}

// branch
#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Branch {
    // hash-linked blocks
    // begin at the root, end at the leaf
    branch: Vec<Block>,
}

// leaves
#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Leaves {
    pub leaves: HashSet<BlockHash>,
}
