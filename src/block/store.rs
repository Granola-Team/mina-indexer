use super::{precomputed::PrecomputedBlock, BlockHash};

pub trait BlockStore {
    fn add_block(&self, block: &PrecomputedBlock) -> anyhow::Result<()>;
    fn get_block(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PrecomputedBlock>>;
}