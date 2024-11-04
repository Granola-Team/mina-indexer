use mina_db::blocks;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    print!("Starting...");

    let _ = tokio::spawn(async move {
        let _ = blocks::run("/Users/jonathan/.mina-indexer/mina-indexer-dev/blocks").await;
    })
    .await;

    Ok(())
}
