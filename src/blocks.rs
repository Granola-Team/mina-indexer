use std::error::Error;

use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub async fn read_block_log(log_path: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    let mut log_file = File::open(log_path).await?;
    let mut contents = Vec::new();

    log_file.read_to_end(&mut contents).await?;

    let block_log = serde_json::from_slice(&contents)?;

    Ok(block_log)
}
