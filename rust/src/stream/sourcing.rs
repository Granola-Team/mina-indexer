use super::{payloads::GenesisBlockPayload, shared_publisher::SharedPublisher};
use crate::{
    stream::events::{Event, EventType},
    utility::extract_height_and_hash,
};
use anyhow::Result;
use std::{cmp::Ordering, fs, path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::broadcast;

pub fn publish_genesis_block(shared_publisher: &Arc<SharedPublisher>) -> Result<()> {
    shared_publisher.publish(Event {
        event_type: EventType::GenesisBlock,
        payload: sonic_rs::to_string(&GenesisBlockPayload::new()).unwrap(),
    });

    Ok(())
}

pub async fn publish_block_dir_paths(
    blocks_dir: PathBuf,
    shared_publisher: &Arc<SharedPublisher>,
    mut shutdown_receiver: broadcast::Receiver<()>,
) -> Result<()> {
    let mut entries: Vec<PathBuf> = fs::read_dir(blocks_dir)?
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .map(|e| e.path())
        .collect();

    // Sort entries by the extracted number and hash
    entries.sort_by(|a, b| {
        let (a_num, a_hash) = extract_height_and_hash(a);
        let (b_num, b_hash) = extract_height_and_hash(b);

        match a_num.cmp(&b_num) {
            Ordering::Equal => a_hash.cmp(b_hash), // Fallback to hash comparison
            other => other,
        }
    });

    let publisher_handle = tokio::spawn({
        let shared_publisher = Arc::clone(shared_publisher);
        async move {
            for entry in entries {
                let path = entry.as_path();
                shared_publisher.publish(Event {
                    event_type: EventType::PrecomputedBlockPath,
                    payload: path.to_str().map(ToString::to_string).unwrap_or_default(),
                });

                tokio::time::sleep(Duration::from_millis(50)).await; // Adjust duration as needed

                if shutdown_receiver.try_recv().is_ok() {
                    println!("Shutdown signal received. Stopping publishing...");
                    break;
                }
            }

            println!("Finished publishing files. Waiting for shutdown signal...");
        }
    });

    if let Err(e) = publisher_handle.await {
        eprintln!("Error in publisher task: {:?}", e);
    }

    Ok(())
}
