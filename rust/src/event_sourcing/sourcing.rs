use super::{
    genesis_ledger_models::GenesisLedger,
    payloads::{AccountingEntry, AccountingEntryAccountType, AccountingEntryType, DoubleEntryRecordPayload, GenesisBlockPayload, LedgerDestination},
    shared_publisher::SharedPublisher,
};
use crate::{
    constants::FILE_PUBLISHER_ACTOR_ID,
    event_sourcing::{
        events::{Event, EventType},
        payloads::ActorHeightPayload,
    },
    utility::extract_height_and_hash,
};
use anyhow::Result;
use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::broadcast;

pub fn publish_genesis_block(shared_publisher: &Arc<SharedPublisher>) -> Result<()> {
    let payload = GenesisBlockPayload::new();
    shared_publisher.publish(Event {
        event_type: EventType::GenesisBlock,
        payload: sonic_rs::to_string(&payload).unwrap(),
    });

    //B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg
    shared_publisher.publish(Event {
        event_type: EventType::DoubleEntryTransaction,
        payload: sonic_rs::to_string(&DoubleEntryRecordPayload {
            height: 1,
            state_hash: payload.state_hash,
            ledger_destination: LedgerDestination::BlockchainLedger,
            lhs: vec![AccountingEntry {
                counterparty: "MagicMinaForBlock0".to_string(),
                transfer_type: "BlockReward".to_string(),
                account: "B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg".to_string(),
                entry_type: AccountingEntryType::Credit,
                account_type: AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: 1000,
                timestamp: payload.unix_timestamp,
            }],
            rhs: vec![AccountingEntry {
                counterparty: "B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg".to_string(),
                transfer_type: "BlockReward".to_string(),
                account: "MagicMinaForBlock0".to_string(),
                entry_type: AccountingEntryType::Debit,
                account_type: AccountingEntryAccountType::VirtualAddess,
                amount_nanomina: 1000,
                timestamp: payload.unix_timestamp,
            }],
        })
        .unwrap(),
    });

    Ok(())
}

pub fn publish_genesis_ledger_double_entries(shared_publisher: &Arc<SharedPublisher>) -> Result<()> {
    for de in get_genesis_ledger().get_accounting_double_entries() {
        shared_publisher.publish(Event {
            event_type: EventType::DoubleEntryTransaction,
            payload: sonic_rs::to_string(&de).unwrap(),
        });
    }

    Ok(())
}

pub fn publish_exempt_accounts(shared_publisher: &Arc<SharedPublisher>) -> Result<()> {
    for account in get_genesis_ledger().get_accounts() {
        shared_publisher.publish(Event {
            event_type: EventType::PreExistingAccount,
            payload: account,
        });
    }

    Ok(())
}

fn get_genesis_ledger() -> GenesisLedger {
    let file_path = PathBuf::from("./src/data/genesis_ledger.json");

    // Ensure the file exists before testing
    let file_content = std::fs::read_to_string(file_path).expect("Failed to read genesis_ledger.json file");

    sonic_rs::from_str(&file_content).expect("Failed to parse genesis_ledger.json")
}

pub async fn publish_block_dir_paths(
    blocks_dir: PathBuf,
    shared_publisher: &Arc<SharedPublisher>,
    mut shutdown_receiver: broadcast::Receiver<()>,
    root_node: Option<(u64, String)>, // height and state hash
) -> Result<()> {
    let mut millisecond_pause = get_millisecond_pause_from_rate();
    let mut entries = get_block_entries(&blocks_dir).await?;

    // Sort entries by height and hash
    sort_entries(&mut entries);

    // Filter entries based on root node, if provided
    if let Some((root_height, root_state_hash)) = root_node {
        entries = filter_entries_after_root(entries, root_height).await?;
        let root_file = get_root_file(&blocks_dir, root_height, &root_state_hash).await?;
        publish_root_file(shared_publisher, root_file).await?;
    }

    let mut high_priority_subcriber = shared_publisher.subscribe_high_priority();

    let publisher_handle = tokio::spawn({
        let shared_publisher = Arc::clone(shared_publisher);
        async move {
            for (i, entry) in entries.iter().enumerate() {
                let path = entry.as_path();

                publish_block_path(&shared_publisher, path).await.unwrap();
                publish_actor_height(&shared_publisher, path).await.unwrap();

                let running_avg_height_spread = handle_height_spread_event(&mut high_priority_subcriber).await;
                // Actors should keep pace with each other, on average. If their processing height differs to much,
                // the pause should increase
                if running_avg_height_spread > 5.0 {
                    millisecond_pause = (millisecond_pause + 100).min(1_000); // increment pause, to slow down ingestion
                }
                if running_avg_height_spread < 1.0 {
                    millisecond_pause = millisecond_pause.saturating_sub(10).max(50); // decrement pause, to speed up ingestion
                }
                tokio::time::sleep(Duration::from_millis(millisecond_pause)).await;
                if i % 100 == 0 {
                    println!("Pause is currently {millisecond_pause}ms for spread {:.2}", running_avg_height_spread);
                }

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

async fn get_block_entries(blocks_dir: &Path) -> Result<Vec<PathBuf>> {
    let entries: Vec<PathBuf> = fs::read_dir(blocks_dir)?
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

        match a_num.cmp(&b_num) {
            Ordering::Equal => a_hash.cmp(b_hash),
            other => other,
        }
    });
}

async fn filter_entries_after_root(entries: Vec<PathBuf>, root_height: u64) -> Result<Vec<PathBuf>> {
    Ok(entries
        .into_iter()
        .filter(|f| {
            let (height, _) = extract_height_and_hash(f);
            height as u64 > root_height
        })
        .collect())
}

async fn get_root_file(blocks_dir: &Path, root_height: u64, root_state_hash: &str) -> Result<PathBuf> {
    fs::read_dir(blocks_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.is_file() && extract_height_and_hash(path) == (root_height as u32, root_state_hash))
        .ok_or_else(|| anyhow::anyhow!("Expected to find a root file"))
}

async fn publish_root_file(shared_publisher: &Arc<SharedPublisher>, root_file: PathBuf) -> Result<()> {
    shared_publisher.publish(Event {
        event_type: EventType::PrecomputedBlockPath,
        payload: root_file.to_str().unwrap().to_string(),
    });
    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok(())
}

async fn handle_height_spread_event(subscriber: &mut broadcast::Receiver<Event>) -> f64 {
    let mut height_spread: f64 = 0.0; // Default to 0

    // Drain events with a timeout and keep the highest HeightSpread value
    while let Ok(Ok(event)) = tokio::time::timeout(std::time::Duration::from_millis(1), subscriber.recv()).await {
        if event.event_type == EventType::RunningAvgHeightSpread {
            let spread = event.payload.parse::<f64>().unwrap_or(0.0);
            height_spread = spread;
        }
    }

    height_spread
}

async fn publish_block_path(shared_publisher: &Arc<SharedPublisher>, path: &Path) -> Result<()> {
    shared_publisher.publish(Event {
        event_type: EventType::PrecomputedBlockPath,
        payload: path.to_str().map(ToString::to_string).unwrap_or_default(),
    });
    Ok(())
}

async fn publish_actor_height(shared_publisher: &Arc<SharedPublisher>, path: &Path) -> Result<()> {
    let (height, _) = extract_height_and_hash(path);
    shared_publisher.publish(Event {
        event_type: EventType::ActorHeight,
        payload: sonic_rs::to_string(&ActorHeightPayload {
            actor: FILE_PUBLISHER_ACTOR_ID.to_string(),
            height: height as u64,
        })
        .unwrap(),
    });
    Ok(())
}

pub fn get_publish_rate() -> u64 {
    std::env::var("PUBLISH_RATE_PER_SECOND")
        .map(|rate_str| rate_str.parse::<u64>().ok().unwrap())
        .unwrap_or(10)
}

pub fn get_millisecond_pause_from_rate() -> u64 {
    1000u64 / get_publish_rate()
}

#[cfg(test)]
mod sourcing_tests {
    use super::*;
    use crate::event_sourcing::events::EventType;
    use futures::lock::Mutex;
    use tokio::sync::broadcast;

    fn setup_shared_publisher() -> Arc<SharedPublisher> {
        Arc::new(SharedPublisher::new(100_000))
    }

    #[tokio::test]
    async fn test_publish_genesis_block() {
        let shared_publisher = setup_shared_publisher();
        let mut receiver = shared_publisher.subscribe();

        publish_genesis_block(&shared_publisher).unwrap();

        let genesis_block_event = receiver.recv().await.unwrap();
        assert_eq!(genesis_block_event.event_type, EventType::GenesisBlock);

        let transaction_event = receiver.recv().await.unwrap();
        assert_eq!(transaction_event.event_type, EventType::DoubleEntryTransaction);
    }

    #[tokio::test]
    async fn test_publish_genesis_ledger_double_entries() {
        let shared_publisher = setup_shared_publisher();
        let mut receiver = shared_publisher.subscribe();

        publish_genesis_ledger_double_entries(&shared_publisher).unwrap();

        let event = receiver.recv().await.unwrap();
        assert_eq!(event.event_type, EventType::DoubleEntryTransaction);
    }

    #[tokio::test]
    async fn test_publish_exempt_accounts() {
        let shared_publisher = setup_shared_publisher();
        let mut receiver = shared_publisher.subscribe();

        publish_exempt_accounts(&shared_publisher).unwrap();

        let event = receiver.recv().await.unwrap();
        assert_eq!(event.event_type, EventType::PreExistingAccount);
        assert_eq!(event.payload, "B62qmqMrgPshhHKLJ7DqWn1KeizEgga5MuGmWb2bXajUnyivfeMW6JE");
    }

    #[tokio::test]
    async fn test_publish_block_dir_paths() {
        // Create a mock blocks directory
        let blocks_dir = "./src/event_sourcing/test_data/10_mainnet_blocks/";

        let shared_publisher = setup_shared_publisher();
        let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

        let mut receiver = shared_publisher.subscribe();

        // Publish block directory paths
        publish_block_dir_paths(
            PathBuf::from(blocks_dir),
            &shared_publisher,
            shutdown_receiver,
            Some((5, "3NKQUoBfi9vkbuqtDJmSEYBQrcSo4GjwG8bPCiii4yqM8AxEQvtY".to_string())),
        )
        .await
        .unwrap();

        // Verify the root block file event is published
        if let Ok(event) = tokio::time::timeout(std::time::Duration::from_secs(2), receiver.recv()).await {
            let event = event.unwrap();
            assert_eq!(event.event_type, EventType::PrecomputedBlockPath);
            assert!(event.payload.contains("mainnet-5-3NKQUoBfi9vkbuqtDJmSEYBQrcSo4GjwG8bPCiii4yqM8AxEQvtY.json"));
        } else {
            panic!("Did not receive the expected PrecomputedBlockPath event for the root block.");
        }

        // Clean up by sending the shutdown signal
        let _ = shutdown_sender.send(());
    }

    #[tokio::test]
    async fn test_handle_height_spread_event_default_zero_if_no_events() {
        // Create a broadcast channel and a subscriber
        let (_, rx) = broadcast::channel::<Event>(10);
        let subscriber = Arc::new(Mutex::new(rx));

        // Call the function to handle events (no events sent)
        let mut sub = subscriber.lock().await;
        let highest_spread = handle_height_spread_event(&mut sub).await;

        // Assert that the highest spread is 0, as no events were received
        assert_eq!(highest_spread, 0.0);
    }
}
