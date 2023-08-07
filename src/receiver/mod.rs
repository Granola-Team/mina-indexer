use crate::block::precomputed::PrecomputedBlock;

pub mod filesystem;

#[async_trait::async_trait]
pub trait BlockReceiver {
    type BlockSource;
    type Error;
    async fn load_source(&mut self, source: &Self::BlockSource) -> Result<(), Self::Error>;
    async fn recv_block(&mut self) -> Option<Result<PrecomputedBlock, Self::Error>>;
}