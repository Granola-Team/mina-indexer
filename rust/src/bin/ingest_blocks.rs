use anyhow::Result;
use chrono::{DateTime, Utc};
use futures::future::try_join_all;
use mina_indexer::{
    constants::CHANNEL_MESSAGE_CAPACITY,
    stream::{
        shared_publisher::SharedPublisher,
        sourcing::{publish_block_dir_paths, publish_genesis_block},
        subscribe_actors,
    },
};
use std::{env, path::PathBuf, sync::Arc, time::Duration};
use tokio::{signal, sync::broadcast};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting...");

    // Create a shutdown channel
    let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

    // Path to the blocks directory
    let blocks_dir = env::var("BLOCKS_DIR")
        .map(PathBuf::from)
        .expect("BLOCKS_DIR environment variable must be present and valid");

    let shared_publisher = Arc::new(SharedPublisher::new(CHANNEL_MESSAGE_CAPACITY));

    // Spawn tasks
    let process_handle = spawn_actor_subscribers(Arc::clone(&shared_publisher), shutdown_receiver.resubscribe());

    // Publish genesis block after a brief delay to allow initialization
    tokio::time::sleep(Duration::from_secs(1)).await;
    publish_genesis_block(&shared_publisher)?;
    let publish_handle = spawn_block_publisher(blocks_dir, Arc::clone(&shared_publisher), shutdown_receiver.resubscribe());
    let monitor_handle = spawn_monitor(Arc::clone(&shared_publisher), shutdown_receiver.resubscribe());

    // Wait for SIGINT to trigger shutdown
    signal::ctrl_c().await?;
    println!("SIGINT received, sending shutdown signal...");

    // Send the shutdown signal
    let _ = shutdown_sender.send(());

    // Await all tasks
    try_join_all(vec![process_handle, publish_handle, monitor_handle]).await?;

    Ok(())
}

/// Spawn a task for subscribing actors to events
fn spawn_actor_subscribers(shared_publisher: Arc<SharedPublisher>, shutdown_receiver: broadcast::Receiver<()>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(e) = subscribe_actors(&shared_publisher, shutdown_receiver).await {
            eprintln!("Error in actor subscription: {:?}", e);
        }
    })
}

/// Spawn a task for publishing block paths
fn spawn_block_publisher(
    blocks_dir: PathBuf,
    shared_publisher: Arc<SharedPublisher>,
    shutdown_receiver: broadcast::Receiver<()>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(e) = publish_block_dir_paths(blocks_dir, &shared_publisher, shutdown_receiver).await {
            eprintln!("Error publishing block paths: {:?}", e);
        }
    })
}

/// Spawn a task for monitoring system metrics with a shutdown signal
fn spawn_monitor(shared_publisher: Arc<SharedPublisher>, mut shutdown_receiver: broadcast::Receiver<()>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_receiver.recv() => {
                    println!("Shutdown signal received, terminating monitor task.");
                    break;
                }
                _ = tokio::time::sleep(Duration::from_secs(60)) => {
                    let now: DateTime<Utc> = Utc::now();
                    println!(
                        "{} Messages published: {}. Database inserts: {}. Ratio: {}",
                        now.to_rfc3339(),
                        shared_publisher.buffer_size(),
                        shared_publisher.database_inserts(),
                        if shared_publisher.database_inserts() > 0 {
                            format!("{:.2}", shared_publisher.buffer_size() as f64 / shared_publisher.database_inserts() as f64)
                        } else {
                            "n/a".to_string()
                        }
                    );
                }
            }
        }
    })
}
