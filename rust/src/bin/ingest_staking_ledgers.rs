use anyhow::Result;
use mina_indexer::{check_or_create_db_schema, staking};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder().with_max_level(Level::INFO).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    check_or_create_db_schema()?;

    staking::run("/Users/jonathan/.mina-indexer/mina-indexer-dev/staking")
}
