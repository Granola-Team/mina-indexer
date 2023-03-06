pub mod parser;
pub mod precomputed;
pub mod store;

// block
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Block {
    pub parent_hash: BlockHash,
    pub state_hash: BlockHash,
    pub global_slot: u32,
    // TODO block
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct BlockHash {
    pub block_hash: String,
}
