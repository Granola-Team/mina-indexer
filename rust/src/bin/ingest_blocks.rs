use mina_indexer::stream::process_blocks_dir;
use std::{path::PathBuf, str::FromStr};
use tokio::{signal, sync::broadcast};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("Starting...");

    // Create a shutdown channel
    let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

    // Path to the blocks directory
    let blocks_dir = PathBuf::from_str("src/stream/test_data/100_mainnet_blocks").expect("blocks dir to be present");

    // Spawn the process_blocks_dir task with the shutdown receiver
    let process_handle = tokio::spawn(async move {
        let _ = process_blocks_dir(blocks_dir, shutdown_receiver).await;
    });

    // Wait indefinitely for Ctrl+C to trigger shutdown
    signal::ctrl_c().await?;
    println!("SIGINT received, sending shutdown signal...");

    // Send the shutdown signal to terminate the process_blocks_dir task
    let _ = shutdown_sender.send(());

    // Wait for process_blocks_dir to shut down gracefully
    if let Err(e) = process_handle.await {
        eprintln!("Error while awaiting process_blocks_dir: {:?}", e);
    } else {
        println!("process_blocks_dir has shut down gracefully.");
    }

    Ok(())
}
