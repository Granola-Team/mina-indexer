use mina_db::blocks;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // a builder for `FmtSubscriber`.
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::INFO)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // let _ = tokio::spawn(async move {
    //     let _ = staking::run("/Users/jonathan/.mina-indexer/mina-indexer-dev/staking").await;
    // })
    // .await;

    let _ = tokio::spawn(async move {
        let _ = blocks::run("/Users/jonathan/.mina-indexer/mina-indexer-dev/blocks").await;
    })
    .await;

    Ok(())
}
