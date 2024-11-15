use crate::{stream::actors::blockchain_tree_builder_actor::BlockchainTreeBuilderActor, utility::extract_height_and_hash};
use actors::{
    accounting_actor::AccountingActor, berkeley_block_parser_actor::BerkeleyBlockParserActor, best_block_actor::BestBlockActor,
    block_ancestor_actor::BlockAncestorActor, block_canonicity_actor::BlockCanonicityActor, block_summary_actor::BlockSummaryActor,
    block_summary_persistence_actor::BlockSummaryPersistenceActor, coinbase_transfer_actor::CoinbaseTransferActor, fee_transfer_actor::FeeTransferActor,
    fee_transfer_via_coinbase_actor::FeeTransferViaCoinbaseActor, internal_command_canonicity_actor::InternalCommandCanonicityActor,
    internal_command_persistence_actor::InternalCommandPersistenceActor, mainnet_block_parser_actor::MainnetBlockParserActor,
    pcb_path_actor::PCBBlockPathActor, snark_canonicity_summary_actor::SnarkCanonicitySummaryActor,
    snark_summary_persistence_actor::SnarkSummaryPersistenceActor, snark_work_actor::SnarkWorkSummaryActor, transition_frontier_actor::TransitionFrontierActor,
    user_command_actor::UserCommandActor, user_command_canonicity_actor::UserCommandCanonicityActor,
    user_command_persistence_actor::UserCommandPersistenceActor, Actor,
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
    shared_publisher: &Arc<SharedPublisher>,
    mut shutdown_receiver: broadcast::Receiver<()>, // Accept shutdown_receiver as a parameter
) -> anyhow::Result<()> {
    println!("Starting process_blocks_dir...");

    let block_persistence_actor = BlockSummaryPersistenceActor::new(Arc::clone(shared_publisher)).await;
    let snark_persistence_actor = SnarkSummaryPersistenceActor::new(Arc::clone(shared_publisher)).await;
    let user_command_persistence_actor_m_0 = UserCommandPersistenceActor::new(Arc::clone(shared_publisher), 0).await;
    let user_command_persistence_actor_m_1 = UserCommandPersistenceActor::new(Arc::clone(shared_publisher), 1).await;
    let user_command_persistence_actor_m_2 = UserCommandPersistenceActor::new(Arc::clone(shared_publisher), 2).await;
    let user_command_persistence_actor_m_3 = UserCommandPersistenceActor::new(Arc::clone(shared_publisher), 3).await;
    let user_command_persistence_actor_m_4 = UserCommandPersistenceActor::new(Arc::clone(shared_publisher), 4).await;
    let internal_command_persistence_actor = InternalCommandPersistenceActor::new(Arc::clone(shared_publisher)).await;

    // Define actors
    let actors: Vec<Arc<dyn Actor + Send + Sync>> = vec![
        Arc::new(PCBBlockPathActor::new(Arc::clone(shared_publisher))),
        Arc::new(BerkeleyBlockParserActor::new(Arc::clone(shared_publisher))),
        Arc::new(MainnetBlockParserActor::new(Arc::clone(shared_publisher))),
        Arc::new(BlockAncestorActor::new(Arc::clone(shared_publisher))),
        Arc::new(BlockchainTreeBuilderActor::new(Arc::clone(shared_publisher))),
        Arc::new(BlockCanonicityActor::new(Arc::clone(shared_publisher))),
        Arc::new(BestBlockActor::new(Arc::clone(shared_publisher))),
        Arc::new(TransitionFrontierActor::new(Arc::clone(shared_publisher))),
        Arc::new(BlockSummaryActor::new(Arc::clone(shared_publisher))),
        Arc::new(SnarkWorkSummaryActor::new(Arc::clone(shared_publisher))),
        Arc::new(SnarkCanonicitySummaryActor::new(Arc::clone(shared_publisher))),
        Arc::new(UserCommandActor::new(Arc::clone(shared_publisher))),
        Arc::new(UserCommandCanonicityActor::new(Arc::clone(shared_publisher))),
        Arc::new(CoinbaseTransferActor::new(Arc::clone(shared_publisher))),
        Arc::new(FeeTransferViaCoinbaseActor::new(Arc::clone(shared_publisher))),
        Arc::new(FeeTransferActor::new(Arc::clone(shared_publisher))),
        Arc::new(InternalCommandCanonicityActor::new(Arc::clone(shared_publisher))),
        Arc::new(AccountingActor::new(Arc::clone(shared_publisher))),
        Arc::new(block_persistence_actor),
        Arc::new(snark_persistence_actor),
        Arc::new(user_command_persistence_actor_m_0),
        Arc::new(user_command_persistence_actor_m_1),
        Arc::new(user_command_persistence_actor_m_2),
        Arc::new(user_command_persistence_actor_m_3),
        Arc::new(user_command_persistence_actor_m_4),
        Arc::new(internal_command_persistence_actor),
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
            Ordering::Equal => a_hash.cmp(b_hash), // Fallback to hash comparison
            other => other,
        }
    });

    let init_actor = tokio::spawn({
        let shared_publisher = Arc::clone(shared_publisher);
        async move {
            // Iterate over files in the directory and publish events
            for entry in entries {
                let path = entry.as_path();
                shared_publisher.publish(Event {
                    event_type: events::EventType::PrecomputedBlockPath,
                    payload: path.to_str().map(ToString::to_string).unwrap_or_default(),
                });
                tokio::task::yield_now().await;
            }

            println!("Finished publishing files. Waiting for shutdown signal...");
        }
    });
    actor_handles.push(init_actor);

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
    use crate::stream::{events::EventType, payloads::*};
    use std::{collections::HashMap, path::PathBuf, str::FromStr};
    use tokio::{sync::broadcast, time::Duration};

    // Create a shutdown channel for the test
    let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

    // Path to the directory with 100 mainnet block files
    let blocks_dir = PathBuf::from_str("./src/stream/test_data/100_mainnet_blocks").expect("Directory with mainnet blocks should exist");

    let shared_publisher = Arc::new(SharedPublisher::new(100_000)); // Initialize publisher
    let mut receiver = shared_publisher.subscribe();

    // Spawn the task to process blocks
    let process_handle = tokio::spawn({
        let shared_publisher = Arc::clone(&shared_publisher);
        async move {
            process_blocks_dir(blocks_dir, &shared_publisher, shutdown_receiver).await.unwrap();
        }
    });

    // Wait a short duration for some events to be processed, then trigger shutdown
    tokio::time::sleep(Duration::from_secs(6)).await;
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
            EventType::InternalCommand => {
                if let Ok(InternalCommandPayload { internal_command_type, .. }) = sonic_rs::from_str(&event.payload) {
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
    assert_eq!(event_counts.get(&EventType::BlockSummary).cloned().unwrap(), paths_plus_genesis_count);
    assert_eq!(event_counts.get(&EventType::UserCommandSummary).cloned().unwrap(), number_of_user_commands);

    assert!(event_counts.get(&EventType::BestBlock).cloned().unwrap() > length_of_chain);
    assert!(event_counts.get(&EventType::BestBlock).cloned().unwrap() < paths_count);
    assert!(!event_counts.contains_key(&EventType::TransitionFrontier));

    assert_eq!(internal_command_counts.get(&InternalCommandType::Coinbase).cloned().unwrap(), paths_count);
    assert!(!internal_command_counts.contains_key(&InternalCommandType::FeeTransfer));
    assert!(!internal_command_counts.contains_key(&InternalCommandType::FeeTransferViaCoinbase));

    // Best Block & Last canonical update:
    assert_eq!(last_best_block.clone().unwrap().height, length_of_chain as u64);
    assert_eq!(&last_best_block.unwrap().state_hash, "3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4");

    Ok(())
}

#[tokio::test]
async fn test_process_blocks_dir_canonical_updates() -> anyhow::Result<()> {
    use crate::stream::{events::EventType, payloads::BlockCanonicityUpdatePayload};
    use std::{path::PathBuf, str::FromStr};
    use tokio::{sync::broadcast, time::Duration};

    // Create a shutdown channel for the test
    let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

    // Path to the directory with 10 mainnet block files
    let blocks_dir = PathBuf::from_str("./src/stream/test_data/10_mainnet_blocks").expect("Directory with mainnet blocks should exist");

    let shared_publisher = Arc::new(SharedPublisher::new(100_000)); // Initialize publisher
    let mut receiver = shared_publisher.subscribe();

    // Spawn the task to process blocks
    let process_handle = tokio::spawn({
        let shared_publisher = Arc::clone(&shared_publisher);
        async move {
            process_blocks_dir(blocks_dir, &shared_publisher, shutdown_receiver).await.unwrap();
        }
    });

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
