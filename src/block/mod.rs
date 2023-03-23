use mina_serialization_types::{common::Base58EncodableVersionedType, v1::HashV1, version_bytes};
use std::fmt::{Debug, Formatter, Result};

use self::precomputed::PrecomputedBlock;

pub mod parser;
pub mod precomputed;
pub mod store;

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct Block {
    pub parent_hash: BlockHash,
    pub state_hash: BlockHash,
    pub height: u32,
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct BlockHash {
    pub block_hash: String,
}

impl BlockHash {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        let block_hash = unsafe { String::from_utf8_unchecked(Vec::from(bytes)) };
        Self { block_hash }
    }

    pub fn from_hashv1(hashv1: HashV1) -> Self {
        let versioned: Base58EncodableVersionedType<{ version_bytes::STATE_HASH }, _> =
            hashv1.into();
        Self {
            block_hash: versioned.to_base58_string().unwrap(),
        }
    }

    pub fn previous_state_hash(block: &PrecomputedBlock) -> Self {
        Self::from_hashv1(block.protocol_state.previous_state_hash.clone())
    }
}

impl Block {
    pub fn from_precomputed(precomputed_block: &PrecomputedBlock, height: u32) -> Self {
        let parent_hash =
            BlockHash::from_hashv1(precomputed_block.protocol_state.previous_state_hash.clone());
        let state_hash = BlockHash {
            block_hash: precomputed_block.state_hash.clone(),
        };
        Self {
            parent_hash,
            state_hash,
            height,
        }
    }
}

impl Debug for Block {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "\nBlock {{\n  height: {:?},\n  state:  {:?},\n parent:  {:?} }}",
            self.height, self.state_hash, self.parent_hash
        )
    }
}

impl Debug for BlockHash {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "BlockHash {{ {:?} }}", self.block_hash)
    }
}
