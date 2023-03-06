use self::precomputed::PrecomputedBlock;

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

impl BlockHash {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        let block_hash = unsafe { String::from_utf8_unchecked(Vec::from(bytes)) };
        Self { block_hash }
    }
}

impl Block {
    pub fn from_precomputed(precomputed_block: &PrecomputedBlock) -> Self {
        let parent_hash = BlockHash::from_bytes(
            precomputed_block
                .protocol_state
                .previous_state_hash
                .clone()
                .inner(),
        );
        let state_hash = BlockHash {
            block_hash: precomputed_block.state_hash.clone(),
        };
        let global_slot = 0;
        Self {
            parent_hash,
            state_hash,
            global_slot,
        }
    }
}
