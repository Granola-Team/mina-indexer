use mina_db::{blocks, staking};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // tokio::spawn(async move {
    //     blocks::run("/Users/jonathan/.mina-indexer/mina-indexer-dev/blocks-9999").await;
    // });

    let _ = tokio::spawn(async move {
        let _ = staking::run("/Users/jonathan/.mina-indexer/mina-indexer-dev/staking").await;
    })
    .await;

    Ok(())
}
