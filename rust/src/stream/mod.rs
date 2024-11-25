use crate::stream::actors::blockchain_tree_builder_actor::BlockchainTreeBuilderActor;
use actors::{
    accounting_actor::AccountingActor, berkeley_block_parser_actor::BerkeleyBlockParserActor, best_block_actor::BestBlockActor,
    block_ancestor_actor::BlockAncestorActor, block_canonicity_actor::BlockCanonicityActor, block_confirmations_actor::BlockConfirmationsActor,
    block_log_actor::BlockLogActor, canonical_block_log_actor::CanonicalBlockLogActor,
    canonical_block_log_persistence_actor::CanonicalBlockLogPersistenceActor, canonical_internal_command_log_actor::CanonicalInternalCommandLogActor,
    canonical_internal_command_log_persistence_actor::CanonicalInternalCommandLogPersistenceActor,
    canonical_user_command_log_actor::CanonicalUserCommandLogActor, canonical_user_command_persistence_actor::CanonicalUserCommandPersistenceActor,
    coinbase_transfer_actor::CoinbaseTransferActor, fee_transfer_actor::FeeTransferActor, fee_transfer_via_coinbase_actor::FeeTransferViaCoinbaseActor,
    ledger_actor::LedgerActor, mainnet_block_parser_actor::MainnetBlockParserActor, monitor_actor::MonitorActor, new_account_actor::NewAccountActor,
    pcb_path_actor::PCBBlockPathActor, snark_canonicity_summary_actor::SnarkCanonicitySummaryActor,
    snark_summary_persistence_actor::SnarkSummaryPersistenceActor, snark_work_actor::SnarkWorkSummaryActor, staking_accounting_actor::StakingAccountingActor,
    staking_ledger_actor::StakingLedgerActor, user_command_log_actor::UserCommandLogActor, Actor,
};
use events::Event;
use futures::future::try_join_all;
use shared_publisher::SharedPublisher;
use std::{sync::Arc, time::Duration};
use tokio::{sync::broadcast, task};

mod actors;
pub mod berkeley_block_models;
pub mod canonical_items_manager;
pub mod db_logger;
pub mod events;
pub mod genesis_ledger_models;
pub mod mainnet_block_models;
pub mod models;
pub mod payloads;
pub mod shared_publisher;
pub mod sourcing;

pub async fn subscribe_actors(
    shared_publisher: &Arc<SharedPublisher>,
    mut shutdown_receiver: broadcast::Receiver<()>, // Accept shutdown_receiver as a parameter
    root_node: Option<(u64, String)>,
) -> anyhow::Result<()> {
    // let snark_persistence_actor = SnarkSummaryPersistenceActor::new(Arc::clone(shared_publisher)).await;
    let canonical_block_log_persistence_actor = CanonicalBlockLogPersistenceActor::new(Arc::clone(shared_publisher), &root_node).await;
    let user_command_persistence_actor = CanonicalUserCommandPersistenceActor::new(Arc::clone(shared_publisher), &root_node).await;
    let internal_command_persistence_actor = CanonicalInternalCommandLogPersistenceActor::new(Arc::clone(shared_publisher), &root_node).await;
    let account_summary_persistence_actor = LedgerActor::new(Arc::clone(shared_publisher), &root_node).await;
    let staking_ledger_actor = StakingLedgerActor::new(Arc::clone(shared_publisher), &root_node).await;
    let new_account_actor = NewAccountActor::new(Arc::clone(shared_publisher), &root_node).await;
    let snark_summary_persistence_actor_m0 = SnarkSummaryPersistenceActor::new(Arc::clone(shared_publisher), &root_node, 0).await;
    let snark_summary_persistence_actor_m1 = SnarkSummaryPersistenceActor::new(Arc::clone(shared_publisher), &root_node, 1).await;
    let snark_summary_persistence_actor_m2 = SnarkSummaryPersistenceActor::new(Arc::clone(shared_publisher), &root_node, 2).await;

    // Define actors
    let actors: Vec<Arc<dyn Actor + Send + Sync>> = vec![
        Arc::new(PCBBlockPathActor::new(Arc::clone(shared_publisher))),
        Arc::new(BerkeleyBlockParserActor::new(Arc::clone(shared_publisher))),
        Arc::new(MainnetBlockParserActor::new(Arc::clone(shared_publisher))),
        Arc::new(BlockAncestorActor::new(Arc::clone(shared_publisher))),
        Arc::new(BlockchainTreeBuilderActor::new(Arc::clone(shared_publisher))),
        Arc::new(BlockCanonicityActor::new(Arc::clone(shared_publisher))),
        Arc::new(BestBlockActor::new(Arc::clone(shared_publisher))),
        Arc::new(BlockLogActor::new(Arc::clone(shared_publisher))),
        Arc::new(SnarkWorkSummaryActor::new(Arc::clone(shared_publisher))),
        Arc::new(SnarkCanonicitySummaryActor::new(Arc::clone(shared_publisher))),
        Arc::new(UserCommandLogActor::new(Arc::clone(shared_publisher))),
        Arc::new(CanonicalUserCommandLogActor::new(Arc::clone(shared_publisher))),
        Arc::new(CoinbaseTransferActor::new(Arc::clone(shared_publisher))),
        Arc::new(FeeTransferViaCoinbaseActor::new(Arc::clone(shared_publisher))),
        Arc::new(FeeTransferActor::new(Arc::clone(shared_publisher))),
        Arc::new(CanonicalInternalCommandLogActor::new(Arc::clone(shared_publisher))),
        Arc::new(AccountingActor::new(Arc::clone(shared_publisher))),
        Arc::new(BlockConfirmationsActor::new(Arc::clone(shared_publisher))),
        Arc::new(CanonicalBlockLogActor::new(Arc::clone(shared_publisher))),
        Arc::new(MonitorActor::new(Arc::clone(shared_publisher))),
        Arc::new(StakingAccountingActor::new(Arc::clone(shared_publisher))),
        Arc::new(snark_summary_persistence_actor_m0),
        Arc::new(snark_summary_persistence_actor_m1),
        Arc::new(snark_summary_persistence_actor_m2),
        Arc::new(user_command_persistence_actor),
        Arc::new(internal_command_persistence_actor),
        Arc::new(account_summary_persistence_actor),
        Arc::new(new_account_actor),
        Arc::new(canonical_block_log_persistence_actor),
        Arc::new(staking_ledger_actor),
    ];

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

async fn setup_actor<A>(mut receiver: broadcast::Receiver<Event>, mut shutdown_rx: broadcast::Receiver<()>, actor: Arc<A>)
where
    A: Actor + Send + Sync + 'static + ?Sized,
{
    loop {
        tokio::select! {
            event = receiver.recv() => {
                match event {
                    Ok(event) => {
                        actor.on_event(event).await;
                    }
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        println!("Actor {} lagged behind, missed {} messages",actor.id(), count);
                    }
                    Err(e) => {
                        println!("{:?}",e)
                    }
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
    use crate::stream::{events::EventType, payloads::*, sourcing::*};
    use std::{collections::HashMap, path::PathBuf, str::FromStr};
    use tokio::{sync::broadcast, time::Duration};

    // Create a shutdown channel for the test
    let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

    let shared_publisher = Arc::new(SharedPublisher::new(100_000)); // Initialize publisher
    let mut receiver = shared_publisher.subscribe();

    // Spawn the task to process blocks
    let process_handle = tokio::spawn({
        let shared_publisher = Arc::clone(&shared_publisher);
        let shutdown_receiver = shutdown_receiver.resubscribe();
        async move {
            subscribe_actors(&shared_publisher, shutdown_receiver, None).await.unwrap();
        }
    });

    let blocks_dir = PathBuf::from_str("./src/stream/test_data/100_mainnet_blocks").expect("Directory with mainnet blocks should exist");
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    publish_genesis_block(&shared_publisher).unwrap();
    publish_block_dir_paths(blocks_dir, &shared_publisher, shutdown_receiver, None).await?;

    // Wait a short duration for some events to be processed, then trigger shutdown
    tokio::time::sleep(Duration::from_secs(1)).await;
    let _ = shutdown_sender.send(());

    // Wait for the task to finish processing
    let _ = process_handle.await;

    // Count each event type received
    let mut event_counts: HashMap<EventType, usize> = HashMap::new();
    let mut internal_command_counts: HashMap<InternalCommandType, usize> = HashMap::new();
    let mut last_best_block: Option<BlockCanonicityUpdatePayload> = None;
    while let Ok(event) = receiver.try_recv() {
        if event.event_type == EventType::BestBlock {
            last_best_block = Some(sonic_rs::from_str(&event.payload).unwrap());
        }
        match event.event_type {
            EventType::InternalCommandLog => {
                if let Ok(InternalCommandLogPayload { internal_command_type, .. }) = sonic_rs::from_str(&event.payload) {
                    *internal_command_counts.entry(internal_command_type).or_insert(0) += 1
                }
            }
            _ => {
                *event_counts.entry(event.event_type).or_insert(0) += 1;
            }
        }
    }

    let paths_count = 165;
    let paths_plus_genesis_count = paths_count + 1;
    let length_of_chain = 100;
    let number_of_user_commands = 247; // hand-calulated

    assert_eq!(event_counts.get(&EventType::PrecomputedBlockPath).cloned().unwrap(), paths_count);
    assert_eq!(event_counts.get(&EventType::MainnetBlockPath).cloned().unwrap(), paths_count);
    assert_eq!(event_counts.get(&EventType::BlockAncestor).cloned().unwrap(), paths_count);
    assert_eq!(event_counts.get(&EventType::NewBlock).cloned().unwrap(), paths_plus_genesis_count);
    assert_eq!(event_counts.get(&EventType::BlockLog).cloned().unwrap(), paths_plus_genesis_count);
    assert_eq!(event_counts.get(&EventType::UserCommandLog).cloned().unwrap(), number_of_user_commands);

    assert!(event_counts.get(&EventType::BestBlock).cloned().unwrap() > length_of_chain);
    assert!(event_counts.get(&EventType::BestBlock).cloned().unwrap() < paths_count);
    assert!(!event_counts.contains_key(&EventType::TransitionFrontier));

    assert_eq!(internal_command_counts.get(&InternalCommandType::Coinbase).cloned().unwrap(), paths_count);
    assert_eq!(internal_command_counts.get(&InternalCommandType::FeeTransfer).cloned().unwrap(), 159); //manual count reveals 161
    assert!(!internal_command_counts.contains_key(&InternalCommandType::FeeTransferViaCoinbase));

    // Best Block & Last canonical update:
    assert_eq!(last_best_block.clone().unwrap().height, length_of_chain as u64);
    assert_eq!(&last_best_block.unwrap().state_hash, "3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4");

    Ok(())
}

#[tokio::test]
async fn test_process_blocks_dir_canonical_updates() -> anyhow::Result<()> {
    use crate::stream::{events::EventType, payloads::BlockCanonicityUpdatePayload, sourcing::*};
    use std::{path::PathBuf, str::FromStr};
    use tokio::{sync::broadcast, time::Duration};

    // Create a shutdown channel for the test
    let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

    let shared_publisher = Arc::new(SharedPublisher::new(100_000)); // Initialize publisher
    let mut receiver = shared_publisher.subscribe();

    // Spawn the task to process blocks
    let process_handle = tokio::spawn({
        let shared_publisher = Arc::clone(&shared_publisher);
        let shutdown_receiver = shutdown_receiver.resubscribe();
        async move {
            subscribe_actors(&shared_publisher, shutdown_receiver, None).await.unwrap();
        }
    });

    let blocks_dir = PathBuf::from_str("./src/stream/test_data/10_mainnet_blocks").expect("Directory with mainnet blocks should exist");
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    publish_genesis_block(&shared_publisher).unwrap();
    publish_block_dir_paths(blocks_dir, &shared_publisher, shutdown_receiver, None).await?;

    // Wait a short duration for some events to be processed, then trigger shutdown
    tokio::time::sleep(Duration::from_secs(5)).await;
    let _ = shutdown_sender.send(());

    // Wait for the task to finish processing
    let _ = process_handle.await;

    // Define the expected canonicity events based on the detailed log entries provided
    let expected_canonical_events = vec![
        // first at height: canonical
        (1u64, String::from("3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ"), true),
        // first at height: canonical
        (2u64, String::from("3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH"), true),
        // first at height: canonical
        (3u64, String::from("3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R"), true),
        // first at height: canonical
        (4u64, String::from("3NL9qBsNibXPm5Nh8cSg5CCqrbzX5VUVY9gJzAbg7EVCF3hfhazG"), true),
        // first at height: canonical
        (5u64, String::from("3NKQUoBfi9vkbuqtDJmSEYBQrcSo4GjwG8bPCiii4yqM8AxEQvtY"), true),
        // first at height: canonical
        (6u64, String::from("3NKqMEewA8gvEiW7So7nZ3DN6tPnmCtHpWuAzADN5ff9wiqkGf45"), true),
        // 3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v has greater last_vrf_output
        (6u64, String::from("3NKqMEewA8gvEiW7So7nZ3DN6tPnmCtHpWuAzADN5ff9wiqkGf45"), false),
        (6u64, String::from("3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v"), true),
        // last_vrf_ouput lexicographically smaller than 3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v
        (6u64, String::from("3NKvdydTvLVDJ9PKAXrisjsXoZQvUy1V2sbComWyB2uyhARCJZ5M"), false),
        // last_vrf_ouput lexicographically smaller than 3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v
        (6u64, String::from("3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v"), false),
        // 3NLM3k3Vk1qs36hZWdbWvi4sqwer3skbgPyHMWrZMBoscNLyjnY2 has greater last_vrf_output
        (6u64, String::from("3NLM3k3Vk1qs36hZWdbWvi4sqwer3skbgPyHMWrZMBoscNLyjnY2"), true),
        // block at height 7 links to 3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v at height 6
        // resulting in branch competition. 3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v at height 6
        // now becomes canonical, despite having smaller last_vrf_output compared to
        // 3NLM3k3Vk1qs36hZWdbWvi4sqwer3skbgPyHMWrZMBoscNLyjnY2
        (6u64, String::from("3NLM3k3Vk1qs36hZWdbWvi4sqwer3skbgPyHMWrZMBoscNLyjnY2"), false),
        (6u64, String::from("3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v"), true),
        (7u64, String::from("3NL7dd6X6316xu6JtJj6cHwAhHrXwZC4SdBU9TUDUUhfAkB8cSoK"), true),
        (7u64, String::from("3NL7dd6X6316xu6JtJj6cHwAhHrXwZC4SdBU9TUDUUhfAkB8cSoK"), false),
        (7u64, String::from("3NLGcwFVQF1p1PrZpusw2fZwBe5HKXGtrGy1Vc4aPkeBtT8nMNUc"), true),
        (8u64, String::from("3NLVZQz4FwFbvW4hejfyRpw5NyP8XvQjhj4wSsCjCKdHNBjwWsPG"), true),
        (9u64, String::from("3NKK3QwQbAgMSmrHq4wpgqEwXp5pd9B18CMQjgYsjKTdq8CAsuM6"), true),
        (9u64, String::from("3NKYjQ6h8xw8RdYvGk8Rc3NnNQHLXjRczUDDZLCXkTJsZFHDhsH6"), false),
        (9u64, String::from("3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw"), false),
        (9u64, String::from("3NKK3QwQbAgMSmrHq4wpgqEwXp5pd9B18CMQjgYsjKTdq8CAsuM6"), false),
        (9u64, String::from("3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw"), true),
        (10u64, String::from("3NKGgTk7en3347KH81yDra876GPAUSoSePrfVKPmwR1KHfMpvJC5"), true),
        (10u64, String::from("3NKGgTk7en3347KH81yDra876GPAUSoSePrfVKPmwR1KHfMpvJC5"), false),
        (10u64, String::from("3NKHYHrqKpDcon6ToV5CLDiheanjshk5gcsNqefnK78phCFTR2aL"), true),
    ];

    // Collect actual BlockCanonicityUpdatePayload events received
    let mut actual_canonical_events = vec![];
    while let Ok(event) = receiver.try_recv() {
        if event.event_type == EventType::BlockCanonicityUpdate {
            let payload: BlockCanonicityUpdatePayload = sonic_rs::from_str(&event.payload).unwrap();
            actual_canonical_events.push((payload.height, payload.state_hash, payload.canonical));
        }
    }

    // Compare the actual and expected events
    assert_eq!(
        actual_canonical_events.len(),
        expected_canonical_events.len(),
        "Mismatch in the number of events"
    );
    assert_eq!(actual_canonical_events, expected_canonical_events, "Events do not match the expected sequence");

    Ok(())
}
