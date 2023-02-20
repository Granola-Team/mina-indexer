// block
#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Block {
    pub parent_hash: BlockHash,
    pub global_slot: u32,
    // TODO block
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct BlockHash {
    pub block_hash: String,
}
