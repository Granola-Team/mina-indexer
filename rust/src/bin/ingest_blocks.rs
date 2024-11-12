use core::time;
use mina_indexer::stream::{process_blocks_dir, shared_publisher::SharedPublisher};
use std::{env, path::PathBuf, str::FromStr, sync::Arc};
use tokio::{signal, sync::broadcast};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("Starting...");

    // Create a shutdown channel
    let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

    // Path to the blocks directory
    let blocks_dir =
        PathBuf::from_str(&env::var("BLOCKS_DIR").expect("expected BLOCKS_DIR environment variable to be present")).expect("expected blocks dir to exist");

    let shared_publisher = Arc::new(SharedPublisher::new(100_000)); // Initialize publisher

    // Spawn the process_blocks_dir task with the shutdown receiver
    let process_handle = {
        let shared_publisher = Arc::clone(&shared_publisher);
        tokio::spawn(async move {
            let shared_publisher = Arc::clone(&shared_publisher);
            let _ = process_blocks_dir(blocks_dir, &shared_publisher, shutdown_receiver).await;
        })
    };

    let monitor_handle = {
        let shared_publisher = Arc::clone(&shared_publisher);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(time::Duration::from_secs(1));
            loop {
                interval.tick().await;
                println!("Current buffer size: {}", shared_publisher.buffer_size());
            }
        })
    };

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

    monitor_handle.abort();

    Ok(())
}
