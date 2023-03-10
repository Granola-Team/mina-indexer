use mina_serialization_types::{v1::HashV1, common::Base58EncodableVersionedType, version_bytes};

use self::precomputed::PrecomputedBlock;

pub mod parser;
pub mod precomputed;
pub mod store;

// block
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Block {
    pub parent_hash: BlockHash,
    pub state_hash: BlockHash,
    // TODO do we need height and global_slot?
    // if we do, I think we need to change our serde stuff to parse the "consensus_state" object too
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

    pub fn from_hashv1(hashv1: HashV1) -> Self {
        let versioned: Base58EncodableVersionedType<{version_bytes::STATE_HASH}, _> = hashv1.into();
        Self { block_hash: versioned.to_base58_string().unwrap() }
    }
}

impl Block {
    pub fn from_precomputed(precomputed_block: &PrecomputedBlock) -> Self {
        let parent_hash = BlockHash::from_hashv1(
            precomputed_block
                .protocol_state
                .previous_state_hash
                .clone(),
        );
        let state_hash = BlockHash {
            block_hash: precomputed_block.state_hash.clone(),
        };
        Self {
            parent_hash,
            state_hash,
        }
    }
}
