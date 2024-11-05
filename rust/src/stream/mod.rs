use actors::{pcb_path_actor::PCBBlockPathActor, Actor};
use events::Event;
use shared_publisher::SharedPublisher;
use std::{fs, path::PathBuf, sync::Arc};
use tokio::{signal, sync::broadcast, task};

mod actors;
pub mod events;
pub mod shared_publisher;

pub async fn process_blocks_dir(blocks_dir: PathBuf) -> anyhow::Result<()> {
    println!("Starting process_blocks_dir...");

    let shared_publisher = Arc::new(SharedPublisher::new(1056)); // Initialize publisher with buffer size

    // Initialize a shutdown channel for graceful termination
    let (shutdown_sender, _) = broadcast::channel(1);
    let pcb_file_actor = Arc::new(PCBBlockPathActor {
        id: "PCBBlockPathActor".to_string(),
        shared_publisher: Arc::clone(&shared_publisher),
    });

    // Start the actor as an async task
    let pcb_file_actor_handle = task::spawn(setup_actor(
        shared_publisher.subscribe(),
        shutdown_sender.subscribe(),
        Arc::clone(&pcb_file_actor),
    ));

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

    // Await the actor handle to ensure it shuts down gracefully
    println!("Waiting for actor to shut down...");
    let _ = tokio::try_join!(pcb_file_actor_handle);
    println!("All actors have been shut down.");
    Ok(())
}

async fn setup_actor<A>(
    mut receiver: broadcast::Receiver<Event>,
    mut shutdown_rx: broadcast::Receiver<()>,
    actor: Arc<A>,
) where
    A: Actor + Send + Sync + 'static,
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
