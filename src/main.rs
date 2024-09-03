use mina_db::{blocks, staking};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // staking must be processed first since *EpochData relies on link to ledger
    // let _ = tokio::spawn(async move {
    //     let _ = staking::run("/Users/jonathan/.mina-indexer/mina-indexer-dev/staking").await;
    // })
    // .await;

    let _ = tokio::spawn(async move {
        let _ = blocks::run("/Users/jonathan/.mina-indexer/mina-indexer-dev/blocks-9999").await;
    })
    .await;

    Ok(())
}
