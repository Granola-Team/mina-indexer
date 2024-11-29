use super::shared_publisher::SharedPublisher;
use crate::event_sourcing::{block_ingestion_actors::Actor, setup_actor};
use futures::future::try_join_all;
use staking_ledger_entry_actor::StakingLedgerEntryActor;
use std::{sync::Arc, time::Duration};
use tokio::{sync::broadcast, task};

pub(crate) mod staking_ledger_entry_actor;

pub async fn subscribe_staking_actors(
    shared_publisher: &Arc<SharedPublisher>,
    mut shutdown_receiver: broadcast::Receiver<()>, // Accept shutdown_receiver as a parameter
) -> anyhow::Result<()> {
    // Define actors
    let actors: Vec<Arc<dyn Actor + Send + Sync>> = vec![Arc::new(StakingLedgerEntryActor::new(Arc::clone(shared_publisher)))];

    let monitor_actors = actors.clone();
    let monitor_shutdown_rx = shutdown_receiver.resubscribe();

    // Spawn tasks for each actor and collect their handles
    let mut actor_handles = Vec::new();
    for actor in actors {
        let receiver = shared_publisher.subscribe();
        let actor_shutdown_rx = shutdown_receiver.resubscribe(); // Use resubscribe for each actor
        let handle = task::spawn(setup_actor(receiver, actor_shutdown_rx, actor));
        actor_handles.push(handle);
    }
    let monitor_handle = tokio::spawn(async move {
        let mut monitor_shutdown_rx = monitor_shutdown_rx;
        loop {
            tokio::select! {
                _ = monitor_shutdown_rx.recv() => {
                    println!("Shutdown signal received, terminating monitor task.");
                    break;
                }
                _ = tokio::time::sleep(Duration::from_secs(60)) => {
                    println!("Actor reports:");
                    for actor in monitor_actors.clone() {
                        actor.report().await;
                    }
                }
            }
        }
    });

    // Wait for the shutdown signal to terminat
    let _ = shutdown_receiver.recv().await;

    // Await all actor handles to ensure they shut down gracefully
    println!("Waiting for all actors to shut down...");
    try_join_all(actor_handles).await?;
    monitor_handle.await?;
    println!("All actors have been shut down.");
    Ok(())
}
