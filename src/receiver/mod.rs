pub mod filesystem;

use crate::block::precomputed::PrecomputedBlock;

#[async_trait::async_trait]
pub trait BlockReceiver {
    async fn recv_block(&mut self) -> Result<Option<PrecomputedBlock>, anyhow::Error>;
}
