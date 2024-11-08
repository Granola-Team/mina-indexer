use mina_indexer::{staking, start};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    start("/Users/jonathan/.mina-indexer/mina-indexer-dev/staking", staking::run).await
}
