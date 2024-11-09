use crate::{stream::actors::blockchain_tree_builder_actor::BlockchainTreeBuilderActor, utility::extract_height_and_hash};
use actors::{
    berkeley_block_parser_actor::BerkeleyBlockParserActor, best_block_actor::BestBlockActor, block_ancestor_actor::BlockAncestorActor,
    block_canonicity_actor::BlockCanonicityActor, mainnet_block_parser_actor::MainnetBlockParserActor, pcb_path_actor::PCBBlockPathActor, Actor,
};
use events::Event;
use futures::future::try_join_all;
use payloads::GenesisBlockPayload;
use shared_publisher::SharedPublisher;
use std::{cmp::Ordering, fs, path::PathBuf, sync::Arc};
use tokio::{sync::broadcast, task};

mod actors;
pub mod berkeley_block_models;
pub mod events;
pub mod mainnet_block_models;
pub mod models;
pub mod payloads;
pub mod shared_publisher;

pub async fn process_blocks_dir(
    blocks_dir: PathBuf,
    mut shutdown_receiver: broadcast::Receiver<()>, // Accept shutdown_receiver as a parameter
) -> anyhow::Result<()> {
    println!("Starting process_blocks_dir...");

    let shared_publisher = Arc::new(SharedPublisher::new(100_000)); // Initialize publisher

    // Define actors
    let actors: Vec<Arc<dyn Actor + Send + Sync>> = vec![
        Arc::new(PCBBlockPathActor::new(Arc::clone(&shared_publisher))),
        Arc::new(BerkeleyBlockParserActor::new(Arc::clone(&shared_publisher))),
        Arc::new(MainnetBlockParserActor::new(Arc::clone(&shared_publisher))),
        Arc::new(BlockAncestorActor::new(Arc::clone(&shared_publisher))),
        Arc::new(BlockchainTreeBuilderActor::new(Arc::clone(&shared_publisher))),
        Arc::new(BlockCanonicityActor::new(Arc::clone(&shared_publisher))),
        Arc::new(BestBlockActor::new(Arc::clone(&shared_publisher))),
    ];

    // Spawn tasks for each actor and collect their handles
    let mut actor_handles = Vec::new();
    for actor in actors {
        let receiver = shared_publisher.subscribe();
        let actor_shutdown_rx = shutdown_receiver.resubscribe(); // Use resubscribe for each actor
        let handle = task::spawn(setup_actor(receiver, actor_shutdown_rx, actor));
        actor_handles.push(handle);
    }

    shared_publisher.publish(Event {
        event_type: events::EventType::GenesisBlock,
        payload: sonic_rs::to_string(&GenesisBlockPayload::new()).unwrap(),
    });

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
            Ordering::Equal => a_hash.cmp(&b_hash), // Fallback to hash comparison
            other => other,
        }
    });

    // Iterate over files in the directory and publish events
    for entry in entries {
        let path = entry.as_path();

        shared_publisher.publish(Event {
            event_type: events::EventType::PrecomputedBlockPath,
            payload: path.to_str().map(ToString::to_string).unwrap_or_default(),
        });
    }

    println!("Finished publishing files. Waiting for shutdown signal...");

    // Wait for the shutdown signal to terminate
    let _ = shutdown_receiver.recv().await;

    // Await all actor handles to ensure they shut down gracefully
    println!("Waiting for all actors to shut down...");
    try_join_all(actor_handles).await?;
    println!("All actors have been shut down.");
    Ok(())
}

async fn setup_actor<A>(mut receiver: broadcast::Receiver<Event>, mut shutdown_rx: broadcast::Receiver<()>, actor: Arc<A>)
where
    A: Actor + Send + Sync + 'static + ?Sized,
{
    loop {
        tokio::select! {
            event = receiver.recv() => {
                if let Ok(event) = event {
                    actor.on_event(event).await;
                }
            },
            _ = shutdown_rx.recv() => {
                actor.shutdown(); // Generalized shutdown call
                break;
            }
        }
    }
}

#[tokio::test]
async fn test_process_blocks_dir_with_mainnet_blocks() -> anyhow::Result<()> {
    use std::{path::PathBuf, str::FromStr};
    use tokio::{sync::broadcast, time::Duration};

    // Create a shutdown channel for the test
    let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

    // Path to the directory with 100 mainnet block files
    let blocks_dir = PathBuf::from_str("../test_data/100_mainnet_blocks").expect("Directory with mainnet blocks should exist");

    // Spawn the process_blocks_dir task with the shutdown receiver
    let process_handle = tokio::spawn(async move {
        let _ = process_blocks_dir(blocks_dir, shutdown_receiver).await;
    });

    // Allow some time for processing
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Send the shutdown signal
    let _ = shutdown_sender.send(());

    // Wait for the task to handle shutdown and finish
    let _ = process_handle.await;

    // TODO: Need to find a way to check log output

    Ok(())
}
