use actors::{block_ancestor_actor::BlockAncestorActor, pcb_path_actor::PCBBlockPathActor, Actor};
use events::Event;
use futures::future::try_join_all;
use shared_publisher::SharedPublisher;
use std::{fs, path::PathBuf, sync::Arc};
use tokio::{signal, sync::broadcast, task};

mod actors;
pub mod berkeley_block_models;
pub mod events;
pub mod mainnet_block_models;
pub mod payloads;
pub mod shared_publisher;

pub async fn process_blocks_dir(blocks_dir: PathBuf) -> anyhow::Result<()> {
    println!("Starting process_blocks_dir...");

    let shared_publisher = Arc::new(SharedPublisher::new(1056)); // Initialize publisher with buffer size

    // Initialize a shutdown channel for graceful termination
    let (shutdown_sender, _) = broadcast::channel(1);

    // Define actors
    let actors: Vec<Arc<dyn Actor + Send + Sync>> = vec![
        Arc::new(PCBBlockPathActor {
            id: "PCBBlockPathActor".to_string(),
            shared_publisher: Arc::clone(&shared_publisher),
        }),
        Arc::new(BlockAncestorActor {
            id: "BlockAncestorActor".to_string(),
            shared_publisher: Arc::clone(&shared_publisher),
        }),
    ];

    // Spawn tasks for each actor and collect their handles
    let mut actor_handles = Vec::new();
    for actor in actors {
        let receiver = shared_publisher.subscribe();
        let shutdown_receiver = shutdown_sender.subscribe();
        let handle = task::spawn(setup_actor(receiver, shutdown_receiver, actor));
        actor_handles.push(handle);
    }

    // Iterate over files in the directory and publish events
    for entry in fs::read_dir(blocks_dir)?
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
    {
        let path = entry.path();

        shared_publisher.publish(Event {
            event_type: events::EventType::PrecomputedBlockPath,
            payload: path.to_str().map(ToString::to_string).unwrap_or_default(),
        });
    }

    println!("Finished publishing files. Waiting indefinitely for SIGINT...");

    // Wait indefinitely for SIGINT signal to terminate
    tokio::select! {
        _ = signal::ctrl_c() => {
            println!("SIGINT received, sending shutdown signal...");
        }
    }

    // Send the shutdown signal to terminate the actors
    println!("Sending shutdown signal to actors.");
    let _ = shutdown_sender.send(());

    // Await all actor handles to ensure they shut down gracefully
    println!("Waiting for all actors to shut down...");
    try_join_all(actor_handles).await?;
    println!("All actors have been shut down.");
    Ok(())
}

async fn setup_actor<A>(
    mut receiver: broadcast::Receiver<Event>,
    mut shutdown_rx: broadcast::Receiver<()>,
    actor: Arc<A>,
) where
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
                println!("Actor {} is shutting down.", actor.id());
                break;
            }
        }
    }
}
