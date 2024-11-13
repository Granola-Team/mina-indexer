use anyhow::Result;
use chrono::{DateTime, Utc};
use mina_indexer::{
    constants::CHANNEL_MESSAGE_CAPACITY,
    stream::{process_blocks_dir, shared_publisher::SharedPublisher},
};
use std::{env, path::PathBuf, str::FromStr, sync::Arc, time::Duration};
use tokio::{signal, sync::broadcast};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("Starting...");

    // Create a shutdown channel
    let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

    // Path to the blocks directory
    let blocks_dir =
        PathBuf::from_str(&env::var("BLOCKS_DIR").expect("expected BLOCKS_DIR environment variable to be present")).expect("expected blocks dir to exist");

    let shared_publisher = Arc::new(SharedPublisher::new(CHANNEL_MESSAGE_CAPACITY)); // Initialize publisher

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
            loop {
                let now: DateTime<Utc> = Utc::now();
                println!("{} Current buffer size: {}", now.to_rfc3339(), shared_publisher.buffer_size());
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        })
    };

    // Wait indefinitely for Ctrl+C to trigger shutdown
    signal::ctrl_c().await?;
    println!("SIGINT received, sending shutdown signal...");

    // Send the shutdown signal to terminate the process_blocks_dir task
    let _ = shutdown_sender.send(());

    process_handle.abort();
    monitor_handle.abort();

    Ok(())
}
