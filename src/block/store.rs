use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    state::Canonicity,
};

pub trait BlockStore {
    fn add_block(&self, block: &PrecomputedBlock) -> anyhow::Result<()>;
    fn get_block(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PrecomputedBlock>>;
}

pub trait CanonicityStore {
    fn get_canonicity(&self, state_hash: &BlockHash) -> anyhow::Result<Option<Canonicity>>;
    fn add_canonical(&self, state_hash: &BlockHash) -> anyhow::Result<()>;
    fn add_orphaned(&self, state_hash: &BlockHash) -> anyhow::Result<()>;
}
