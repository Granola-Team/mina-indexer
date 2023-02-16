use async_trait::*;

use super::BlockLog;

pub mod filesystem_json;

#[async_trait]
pub trait BlockLogReader {
    type LogError;

    async fn next_log(&mut self) -> Result<Option<BlockLog>, Self::LogError>;
}

#[async_trait]
impl BlockLogReader for filesystem_json::FilesystemJSONReader {
    type LogError = filesystem_json::LogError;

    async fn next_log(&mut self) -> Result<Option<BlockLog>, Self::LogError> {
        let mut next_log = None;
        if let Some((state_hash, log_path)) = self.next_log_data().await? {
            let json = filesystem_json::FilesystemJSONReader::read_block_log(log_path).await?;
            next_log = Some(BlockLog { state_hash, json })
        }

        Ok(next_log)
    }
}
