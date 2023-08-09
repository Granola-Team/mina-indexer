use crate::block::precomputed::PrecomputedBlock;

pub mod filesystem;
pub mod google_cloud;

#[async_trait::async_trait]
pub trait BlockReceiver {
    type BlockSource;
    type Error: std::error::Error;
    async fn load_source(&mut self, source: &Self::BlockSource) -> Result<(), Self::Error>;
    async fn recv_block(&mut self) -> Option<Result<PrecomputedBlock, Self::Error>>;
}