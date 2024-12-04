use anyhow::Result;
use chrono::{DateTime, Utc};
use env_logger::Builder;
use futures::future::try_join_all;
use log::{error, info};
use mina_indexer::{
    constants::CHANNEL_MESSAGE_CAPACITY,
    event_sourcing::{
        shared_publisher::SharedPublisher,
        sourcing::{publish_block_dir_paths, publish_exempt_accounts, publish_genesis_block, publish_genesis_ledger_double_entries},
        subscribe_actors,
    },
};
use std::{env, path::PathBuf, sync::Arc, time::Duration};
use tokio::{signal, sync::broadcast};

#[tokio::main]
async fn main() -> Result<()> {
    Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .filter_module("tokio_postgres", log::LevelFilter::Warn)
        .init();
    let args: Vec<String> = env::args().collect();

    let mut root_node = None;
    if args.len() == 3 {
        root_node = Some((args[1].parse::<u64>().unwrap(), args[2].to_string()));
        info!("Starting from canonical root at height {} and state hash {}", &args[1], &args[2]);
    } else {
        info!("Starting from genesis root");
    }
    async_main(root_node).await
}

async fn async_main(root_node: Option<(u64, String)>) -> Result<()> {
    // Create a shutdown channel
    let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

    // Path to the blocks directory
    let blocks_dir = env::var("BLOCKS_DIR")
        .map(PathBuf::from)
        .expect("BLOCKS_DIR environment variable must be present and valid");

    let shared_publisher = Arc::new(SharedPublisher::new(CHANNEL_MESSAGE_CAPACITY));

    // Spawn tasks
    let process_handle = spawn_actor_subscribers(Arc::clone(&shared_publisher), shutdown_receiver.resubscribe(), root_node.clone());

    if root_node.is_none() {
        // Publish genesis block after a brief delay to allow initialization
        tokio::time::sleep(Duration::from_secs(1)).await;
        publish_genesis_ledger_double_entries(&shared_publisher)?;
        // Publish accounts that are excempt from 1 mina account creation fee
        tokio::time::sleep(Duration::from_secs(1)).await;
        publish_exempt_accounts(&shared_publisher)?;
        // Publish the genesis block
        tokio::time::sleep(Duration::from_secs(1)).await;
        publish_genesis_block(&shared_publisher)?;
    }
    tokio::time::sleep(Duration::from_secs(1)).await;

    let publish_handle = spawn_block_publisher(blocks_dir, Arc::clone(&shared_publisher), shutdown_receiver.resubscribe(), root_node);
    let monitor_handle = spawn_monitor(Arc::clone(&shared_publisher), shutdown_receiver.resubscribe());

    // Wait for SIGINT to trigger shutdown
    signal::ctrl_c().await?;
    info!("SIGINT received, sending shutdown signal...");

    // Send the shutdown signal
    let _ = shutdown_sender.send(());

    // Await all tasks
    try_join_all(vec![process_handle, publish_handle, monitor_handle]).await?;

    Ok(())
}

/// Spawn a task for subscribing actors to events
fn spawn_actor_subscribers(
    shared_publisher: Arc<SharedPublisher>,
    shutdown_receiver: broadcast::Receiver<()>,
    root_node: Option<(u64, String)>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(e) = subscribe_actors(&shared_publisher, shutdown_receiver, root_node).await {
            error!("Error in actor subscription: {:?}", e);
        }
    })
}

/// Spawn a task for publishing block paths
fn spawn_block_publisher(
    blocks_dir: PathBuf,
    shared_publisher: Arc<SharedPublisher>,
    shutdown_receiver: broadcast::Receiver<()>,
    root_node: Option<(u64, String)>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(e) = publish_block_dir_paths(blocks_dir, &shared_publisher, shutdown_receiver, root_node).await {
            error!("Error publishing block paths: {:?}", e);
        }
    })
}

/// Spawn a task for monitoring system metrics with a shutdown signal
fn spawn_monitor(shared_publisher: Arc<SharedPublisher>, mut shutdown_receiver: broadcast::Receiver<()>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_receiver.recv() => {
                    info!("Shutdown signal received, terminating monitor task.");
                    break;
                }
                _ = tokio::time::sleep(Duration::from_secs(60)) => {
                    let now: DateTime<Utc> = Utc::now();
                    info!(
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

// #[cfg(test)]
// mod ingest_blocks_tests {
//     use super::*;
//     use anyhow::Result;
//     use mina_indexer::constants::POSTGRES_CONNECTION_STRING;
//     use std::env;
//     use tokio_postgres::{Client, NoTls};

//     async fn setup_client() -> Client {
//         let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
//             .await
//             .expect("Failed to connect to the database");

//         tokio::spawn(async move {
//             if let Err(e) = connection.await {
//                 eprintln!("Database connection error: {}", e);
//             }
//         });

//         client
//     }

//     #[tokio::test]
//     async fn test_ledger_at_height_5000() -> Result<()> {
//         env::set_var("BLOCKS_DIR", "./src/event_sourcing/test_data/5000_mainnet_blocks");

//         let client = setup_client().await;

//         let main_handle = tokio::spawn(async { async_main(None).await });

//         // Allow time for processing to complete
//         tokio::time::sleep(std::time::Duration::from_secs(5 * 60)).await;

//         drop(main_handle);
//         // -- 472 (fee transfer goes to coinbase receiver)

//         // 1. Assert that the number of accounts in the ledger is correct. This includes accounts that did not make it into the canonical chain at any point.
//         let row = client
//             .query_one(
//                 "SELECT count(distinct address) FROM blockchain_ledger WHERE address_type = 'BlockchainAddress'",
//                 &[],
//             )
//             .await
//             .expect("Failed to query database");

//         let count: i64 = row.get(0);

//         assert_eq!(count, 1980, "No data was inserted into the database");
//         println!("Number of rows with address_type='BlockchainAddress': {}", count);

//         Ok(())
//     }
// }
