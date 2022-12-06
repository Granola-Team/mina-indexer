use mina_indexer::{block_stream::BlockStream, constants::GRAPHQL_URL, subchain::SubchainIndexer};

use std::error::Error;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let mut block_stream = BlockStream::new(GRAPHQL_URL).await?;
    let mut subchain_indexer = SubchainIndexer::new();
    while let Some(message) = block_stream.next().await {
        if let Some(block) = message {
            subchain_indexer.add_block(block);
            println!("{}", subchain_indexer.as_dot());
        }
    }
    Ok(())
}
