use crate::block::BlockHash;

pub trait CanonicityStore {
    fn get_max_canonical_blockchain_length(&self) -> anyhow::Result<Option<u32>>;
    fn set_max_canonical_blockchain_length(&self, height: u32) -> anyhow::Result<()>;
    fn add_canonical_block(&self, height: u32, state_hash: &BlockHash) -> anyhow::Result<()>;
    fn get_canonical_hash_at_height(&self, height: u32) -> anyhow::Result<Option<BlockHash>>;
}
