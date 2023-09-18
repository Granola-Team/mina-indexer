use crate::block::precomputed::PrecomputedBlock;

pub mod filesystem;

#[async_trait::async_trait]
pub trait BlockReceiver {
    async fn recv_block(&mut self) -> Result<Option<PrecomputedBlock>, anyhow::Error>;
}
