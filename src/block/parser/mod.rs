use async_trait::async_trait;

use super::precomputed::PrecomputedBlock;

pub mod filesystem;

#[async_trait]
pub trait BlockParser: std::fmt::Debug {
    async fn next(&mut self) -> anyhow::Result<Option<PrecomputedBlock>>;
}
