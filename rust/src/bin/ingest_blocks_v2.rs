use env_logger::Builder;
use log::{error, info};
use mina_indexer::event_sourcing::{
    actors_v2::spawn_actor_dag,
    events::{Event, EventType},
    sourcing::{get_block_entries, sort_entries},
};
use std::{env, path::PathBuf, time::Duration};

#[tokio::main]
async fn main() {
    // 1) Initialize logger
    Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .filter_module("tokio_postgres", log::LevelFilter::Warn)
        .init();

    // 2) Figure out where the block files are
    let blocks_dir = env::var("BLOCKS_DIR")
        .map(PathBuf::from)
        .expect("BLOCKS_DIR environment variable must be present and valid");

    // 3) Spawn your actor DAG, which returns a Sender<Event>
    let sender = spawn_actor_dag();

    // 4) Give the DAG a moment to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 5) Gather and sort your block entries
    let mut entries = get_block_entries(&blocks_dir).await.unwrap();
    sort_entries(&mut entries);

    // 6) We'll spawn the actual processing in a task
    let process_handle = tokio::spawn({
        let sender_clone = sender.clone();
        async move {
            for file in entries {
                // If the actor DAG fails to accept the event, log and continue
                if let Err(err) = sender_clone
                    .send(Event {
                        event_type: EventType::PrecomputedBlockPath,
                        payload: file.to_str().unwrap().to_string(),
                    })
                    .await
                {
                    error!("Failed to send file {}: {}", file.display(), err);
                }
            }
            info!("Done processing all block entries.");
        }
    });

    // 8) Race them using select!
    tokio::select! {
        // If the block-processing finishes first, great!
        _ = process_handle => {
            info!("All blocks processed without interruption.");
        }

        // If SIGINT arrives first, we log and break
        _ = Box::pin(tokio::signal::ctrl_c()) => {
            error!("Received Ctrl-C. Stopping block iteration early.");
            // (The for-loop ends because 'process_handle' is still running,
            // but we won't kill it forcibly. We do rely on the lines below
            // to let the DAG flush.)
        }
    }

    // 9) Give the DAG time to flush any in-flight operations
    info!("Giving some time for the DAG to flush...");
    tokio::time::sleep(Duration::from_secs(3)).await;

    info!("Shutting down gracefully. Goodbye!");
}
