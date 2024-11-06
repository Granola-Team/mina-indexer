use mina_db::stream::process_blocks_dir;
use std::{path::PathBuf, str::FromStr};
use tokio::{signal, sync::broadcast};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("Starting...");

    // Create a shutdown channel
    let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

    // Spawn the process_blocks_dir task with the shutdown receiver
    let blocks_dir =
        PathBuf::from_str("/Users/nathantranquilla/mina-indexer-mnt/mina-indexer-prod/blocks-100")
            .expect("blocks dir to be present");

    let process_handle = tokio::spawn(async move {
        let _ = process_blocks_dir(blocks_dir, shutdown_receiver).await;
    });

    // Wait for either Ctrl+C or process completion
    tokio::select! {
        _ = signal::ctrl_c() => {
            println!("SIGINT received, sending shutdown signal...");
            let _ = shutdown_sender.send(()); // Send shutdown signal
        },
        _ = process_handle => {
            println!("process_blocks_dir completed.");
        }
    }

    Ok(())
}
