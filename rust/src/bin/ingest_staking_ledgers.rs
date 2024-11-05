use mina_db::staking;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("Starting...");
    let _ = tokio::spawn(async move {
        let _ = staking::run("/Users/jonathan/.mina-indexer/mina-indexer-dev/staking").await;
    })
    .await;

    Ok(())
}
