use async_trait::async_trait;

use self::filesystem::FilesystemParser;

use super::precomputed::PrecomputedBlock;

pub mod filesystem;

#[async_trait]
pub trait BlockParser {
    async fn next(&mut self) -> anyhow::Result<Option<PrecomputedBlock>>;
    async fn total_num_blocks(&self) -> u32;
    async fn num_canonical(&self) -> u32;
}