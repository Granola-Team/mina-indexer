use mina_serialization_types::{common::Base58EncodableVersionedType, v1::HashV1, version_bytes};

use self::precomputed::PrecomputedBlock;

pub mod parser;
pub mod precomputed;
pub mod store;

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct Block {
    pub parent_hash: BlockHash,
    pub state_hash: BlockHash,
    pub height: u32,
    pub blockchain_length: Option<u32>,
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct BlockHash(pub String);

impl BlockHash {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        let block_hash = unsafe { String::from_utf8_unchecked(Vec::from(bytes)) };
        Self(block_hash)
    }

    pub fn from_hashv1(hashv1: HashV1) -> Self {
        let versioned: Base58EncodableVersionedType<{ version_bytes::STATE_HASH }, _> =
            hashv1.into();
        Self(versioned.to_base58_string().unwrap())
    }

    pub fn previous_state_hash(block: &PrecomputedBlock) -> Self {
        Self::from_hashv1(block.protocol_state.previous_state_hash.clone())
    }
}

impl Block {
    pub fn from_precomputed(precomputed_block: &PrecomputedBlock, height: u32) -> Self {
        let parent_hash =
            BlockHash::from_hashv1(precomputed_block.protocol_state.previous_state_hash.clone());
        let state_hash = BlockHash(precomputed_block.state_hash.clone());
        Self {
            parent_hash,
            state_hash,
            height,
            blockchain_length: precomputed_block.blockchain_length,
        }
    }
}

impl std::fmt::Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "\nBlock")?;
        writeln!(f, "    height:  {},", self.height)?;
        writeln!(f, "    length:  {:?},", self.blockchain_length)?;
        writeln!(f, "    state:   {:?},", self.state_hash)?;
        writeln!(f, "    parent:  {:?},", self.parent_hash)
    }
}

impl std::fmt::Debug for BlockHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "BlockHash {{ {:?} }}", self.0)
    }
}
