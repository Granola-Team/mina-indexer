use super::{
    genesis_ledger_models::GenesisLedger,
    payloads::{AccountingEntry, AccountingEntryAccountType, AccountingEntryType, DoubleEntryRecordPayload, GenesisBlockPayload, LedgerDestination},
    shared_publisher::SharedPublisher,
};
use crate::{
    constants::FILE_PUBLISHER_ACTOR_ID,
    stream::{
        events::{Event, EventType},
        payloads::ActorHeightPayload,
    },
    utility::extract_height_and_hash,
};
use anyhow::Result;
use std::{cmp::Ordering, fs, path::PathBuf, sync::Arc, time::Duration};
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
    let millisecond_pause = get_millisecond_pause_from_rate();
    let mut entries: Vec<PathBuf> = fs::read_dir(blocks_dir.clone())?
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

    if let Some((root_height, root_state_hash)) = root_node {
        entries = entries
            .into_iter()
            .filter(|f| {
                let (height, _) = extract_height_and_hash(f);
                height as u64 > root_height
            })
            .collect::<Vec<_>>();
        let root_file: PathBuf = fs::read_dir(blocks_dir)?
            .filter_map(|entry| entry.ok()) // Filter out invalid entries
            .map(|entry| entry.path()) // Map to the entry's path
            .find(|path| path.is_file() && extract_height_and_hash(path) == (root_height as u32, &root_state_hash))
            .expect("Expected to find a root file");

        println!("root file: {}", root_file.to_str().unwrap());

        shared_publisher.publish(Event {
            event_type: EventType::PrecomputedBlockPath,
            payload: root_file.to_str().unwrap().to_string(),
        });
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    let publisher_handle = tokio::spawn({
        let shared_publisher = Arc::clone(shared_publisher);
        async move {
            for entry in entries {
                let path = entry.as_path();
                shared_publisher.publish(Event {
                    event_type: EventType::PrecomputedBlockPath,
                    payload: path.to_str().map(ToString::to_string).unwrap_or_default(),
                });

                let (height, _) = extract_height_and_hash(path);

                shared_publisher.publish(Event {
                    event_type: EventType::ActorHeight,
                    payload: sonic_rs::to_string(&ActorHeightPayload {
                        actor: FILE_PUBLISHER_ACTOR_ID.to_string(),
                        height: height as u64,
                    })
                    .unwrap(),
                });

                tokio::time::sleep(Duration::from_millis(millisecond_pause)).await;

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

pub fn get_publish_rate() -> u64 {
    std::env::var("PUBLISH_RATE_PER_SECOND")
        .map(|rate_str| rate_str.parse::<u64>().ok().unwrap())
        .unwrap_or(5)
}

pub fn get_millisecond_pause_from_rate() -> u64 {
    1000u64 / get_publish_rate()
}

#[cfg(test)]
mod sourcing_tests {
    use super::*;
    use crate::stream::events::EventType;
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
        let blocks_dir = "./src/stream/test_data/10_mainnet_blocks/";

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
}
