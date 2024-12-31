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

    let destroy_data = env::var("DESTROY_DATA")
        .ok() // Convert Result<String, VarError> to Option<String>
        .and_then(|val| val.parse::<bool>().ok()) // Try to parse "true"/"false" => Option<bool>
        .unwrap_or(false);

    // 3) Spawn your actor DAG, which returns a Sender<Event>
    let (dag, sender) = spawn_actor_dag(!destroy_data).await;

    // 4) Give the DAG a moment to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 5) Gather and sort your block entries
    let mut entries = get_block_entries(&blocks_dir).await.unwrap();
    sort_entries(&mut entries);

    // 6) Convert entries into an iterator, so we can pick them one-by-one
    let mut entries_iter = entries.into_iter();

    // 7) Prepare ctrl-c future. We'll pin it so we can poll it repeatedly.
    let mut ctrl_c_fut = Box::pin(tokio::signal::ctrl_c());

    // 8) For each file, we `tokio::select!` between:
    //    - sending the file event
    //    - or receiving SIGINT
    info!("Ingesting blocks...");
    loop {
        // Move to the next file or break if none left
        let file = match entries_iter.next() {
            Some(f) => f,
            None => {
                info!("Done processing all block entries.");
                break;
            }
        };

        tokio::select! {
            // If SIGINT arrives first, we break out.
            _ = &mut ctrl_c_fut => {
                error!("Received Ctrl-C. Stopping block iteration early.");
                break;
            }

            // Otherwise, we proceed to send the file event.
            // The `default` branch means "no other future to poll" is ready,
            // so continue to send the file.
            _ = async {
                if let Err(err) = sender
                    .send(Event {
                        event_type: EventType::PrecomputedBlockPath,
                        payload: file.to_str().unwrap().to_string(),
                    })
                    .await
                {
                    error!("Failed to send file {}: {}", file.display(), err);
                }
            } => {
                // do nothing here, just loop again
            }
        }
    }

    // 9) Give the DAG time to flush any in-flight operations
    info!("Giving some time for the DAG to flush...");
    dag.lock().await.wait_until_quiesced().await;

    info!("Shutting down gracefully. Goodbye!");
}
