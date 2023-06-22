use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    state::Canonicity,
};

pub trait BlockStore {
    fn add_block(&self, block: &PrecomputedBlock) -> anyhow::Result<()>;
    fn get_block(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PrecomputedBlock>>;
    fn set_canonicity(&self, state_hash: &BlockHash, canonicity: Canonicity) -> anyhow::Result<()>;
    fn get_canonicity(&self, state_hash: &BlockHash) -> anyhow::Result<Option<Canonicity>>;
}