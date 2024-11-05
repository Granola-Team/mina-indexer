use mina_db::stream::process_blocks_dir;
use std::{path::PathBuf, str::FromStr};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("Starting...");

    tokio::spawn(async move {
        let _ = process_blocks_dir(
            PathBuf::from_str(
                "/Users/nathantranquilla/mina-indexer-mnt/mina-indexer-prod/blocks-100",
            )
            .ok()
            .expect("blocks dir to be present"),
        )
        .await;
    })
    .await?;

    // let _ = tokio::spawn(async move {
    //     let _ =
    // blocks::run("/Users/jonathan/.mina-indexer/mina-indexer-dev/blocks").await;
    // })
    // .await;

    Ok(())
}
