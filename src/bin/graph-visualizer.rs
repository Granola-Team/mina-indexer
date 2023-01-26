use mina_indexer::{block_stream::BlockStream, constants::GRAPHQL_URL, subchain::SubchainIndexer};
use tokio::{fs::File, io::{AsyncWriteExt, AsyncSeekExt}};

use std::{error::Error, env};

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let output_file = env::var("GRAPHVIZ_OUT")?;
    let mut block_stream = BlockStream::new(GRAPHQL_URL).await?;
    let mut subchain_indexer = SubchainIndexer::new();
    let mut dot_file = File::create(&output_file).await?;
    while let Some(message) = block_stream.next().await {
        if let Some(block) = message {
            dot_file.rewind().await?;
            subchain_indexer.add_block(block);
            dot_file.write_all(&subchain_indexer.to_dot().into_bytes()).await?;
        }
    }
    Ok(())
}
