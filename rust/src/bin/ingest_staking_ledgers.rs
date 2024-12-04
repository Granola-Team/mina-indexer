use anyhow::Result;
use futures::future::try_join_all;
use log::{error, info};
use mina_indexer::{
    constants::CHANNEL_MESSAGE_CAPACITY,
    event_sourcing::{
        events::{Event, EventType},
        shared_publisher::SharedPublisher,
        staking_ledger_actors::subscribe_staking_actors,
    },
    utility::extract_height_and_hash,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{signal, sync::broadcast};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

    let staking_ledger_dir = std::env::var("STAKING_LEDGER_DIR")
        .map(PathBuf::from)
        .expect("STAKING_LEDGER_DIR environment variable must be present and valid");

    let shared_publisher = Arc::new(SharedPublisher::new(CHANNEL_MESSAGE_CAPACITY));

    let mut staking_ledgers = get_staking_ledgers(&staking_ledger_dir)?;

    sort_entries(&mut staking_ledgers);

    let shared_publisher_clone = Arc::clone(&shared_publisher);

    let actors_handle = tokio::spawn(async move {
        if let Err(e) = subscribe_staking_actors(&shared_publisher, shutdown_receiver.resubscribe()).await {
            error!("Error in actor subscription: {:?}", e);
        }
    });

    tokio::time::sleep(Duration::from_secs(1)).await;

    for staking_ledger in staking_ledgers {
        shared_publisher_clone.publish(Event {
            event_type: EventType::StakingLedgerFilePath,
            payload: staking_ledger.to_str().unwrap().to_string(),
        });
        let (height, _) = extract_height_and_hash(staking_ledger.as_path());
        info!("Published Staking Ledger {height}");
        tokio::time::sleep(Duration::from_secs(10)).await;
    }

    signal::ctrl_c().await?;
    info!("SIGINT received, sending shutdown signal...");

    // Send the shutdown signal
    let _ = shutdown_sender.send(());
    try_join_all([actors_handle]).await.unwrap();

    Ok(())
}

fn get_staking_ledgers(staking_ledgers_dir: &Path) -> Result<Vec<PathBuf>> {
    let entries: Vec<PathBuf> = std::fs::read_dir(staking_ledgers_dir)?
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .map(|e| e.path())
        .collect();
    Ok(entries)
}

fn sort_entries(entries: &mut [PathBuf]) {
    entries.sort_by(|a, b| {
        let (a_num, a_hash) = extract_height_and_hash(a);
        let (b_num, b_hash) = extract_height_and_hash(b);

        a_num.cmp(&b_num).then_with(|| a_hash.cmp(b_hash))
    });
}
