use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::stream::{models::Height, payloads::*};
use anyhow::Result;
use async_trait::async_trait;
use futures::lock::Mutex;
use std::{
    collections::{HashMap, VecDeque},
    sync::{atomic::AtomicUsize, Arc},
};

pub struct UserCommandCanonicityActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
    pub block_canonicity_queue: Arc<Mutex<VecDeque<BlockCanonicityUpdatePayload>>>,
    pub user_commands: Arc<Mutex<HashMap<Height, Vec<UserCommandSummaryPayload>>>>,
}

impl UserCommandCanonicityActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "UserCommandCanonicityActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
            block_canonicity_queue: Arc::new(Mutex::new(VecDeque::new())),
            user_commands: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn process_user_commands(&self) -> Result<(), &'static str> {
        let mut queue = self.block_canonicity_queue.lock().await;

        while let Some(update) = queue.pop_front() {
            let snarks = self.user_commands.lock().await;
            if let Some(entries) = snarks.get(&Height(update.height)) {
                for entry in entries {
                    let payload = UserCommandCanonicityPayload {
                        height: entry.height,
                        state_hash: entry.state_hash.to_string(),
                        timestamp: entry.timestamp,
                        txn_type: entry.txn_type.clone(),
                        status: entry.status.clone(),
                        sender: entry.sender.to_string(),
                        receiver: entry.receiver.to_string(),
                        nonce: entry.nonce,
                        fee_nanomina: entry.fee_nanomina,
                        fee_payer: entry.fee_payer.to_string(),
                        amount_nanomina: entry.amount_nanomina,
                        canonical: update.canonical,
                        was_canonical: update.was_canonical,
                    };
                    self.publish(Event {
                        event_type: EventType::UserCommandCanonicityUpdate,
                        payload: sonic_rs::to_string(&payload).unwrap(),
                    });
                }
            } else {
                queue.push_back(update);
                drop(queue);
                break;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Actor for UserCommandCanonicityActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.events_published
    }
    async fn handle_event(&self, event: Event) {
        match event.event_type {
            EventType::BlockCanonicityUpdate => {
                let mut queue = self.block_canonicity_queue.lock().await;
                queue.push_back(sonic_rs::from_str(&event.payload).unwrap());
                drop(queue);
                self.process_user_commands()
                    .await
                    .expect("Expected to published user command canonicity updates");
            }
            EventType::UserCommandSummary => {
                let event_payload: UserCommandSummaryPayload = sonic_rs::from_str(&event.payload).unwrap();
                let mut user_commands = self.user_commands.lock().await;
                user_commands.entry(Height(event_payload.height)).or_insert_with(Vec::new).push(event_payload);
                drop(user_commands);
                self.process_user_commands().await.expect("Expected to published snark canonicity updates");
            }
            EventType::TransitionFrontier => {
                let height: u64 = sonic_rs::from_str(&event.payload).unwrap();
                let mut user_commands = self.user_commands.lock().await;
                user_commands.retain(|key, _| key.0 > height);
                drop(user_commands);
            }
            _ => return,
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[tokio::test]
async fn test_user_command_canonicity_actor_processes_user_command_updates() -> anyhow::Result<()> {
    use crate::stream::{
        mainnet_block_models::CommandStatus,
        payloads::{BlockCanonicityUpdatePayload, UserCommandCanonicityPayload, UserCommandSummaryPayload},
    };
    use std::sync::atomic::Ordering;
    use tokio::time::timeout;

    // Set up a shared publisher and instantiate the actor
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = UserCommandCanonicityActor::new(Arc::clone(&shared_publisher));

    // Subscribe to capture any output events from the actor
    let mut receiver = shared_publisher.subscribe();

    // Define a sample UserCommandSummaryPayload
    let user_command_payload = UserCommandSummaryPayload {
        height: 10,
        state_hash: "sample_hash".to_string(),
        timestamp: 123456,
        txn_type: crate::stream::mainnet_block_models::CommandType::Payment,
        status: CommandStatus::Applied,
        sender: "sender_public_key".to_string(),
        receiver: "receiver_public_key".to_string(),
        nonce: 1,
        fee_nanomina: 10_000_000,
        amount_nanomina: 500_000_000,
        ..Default::default()
    };

    // Send a UserCommandSummary event to populate the user_commands map
    actor
        .handle_event(Event {
            event_type: EventType::UserCommandSummary,
            payload: sonic_rs::to_string(&user_command_payload).unwrap(),
        })
        .await;

    // Send a BlockCanonicityUpdate event to trigger processing
    let canonical_update_payload = BlockCanonicityUpdatePayload {
        height: 10,
        canonical: true,
        state_hash: "sample_hash".to_string(),
        was_canonical: false,
    };

    actor
        .handle_event(Event {
            event_type: EventType::BlockCanonicityUpdate,
            payload: sonic_rs::to_string(&canonical_update_payload).unwrap(),
        })
        .await;

    // Confirm that the UserCommandCanonicityUpdate event was published with correct data
    let published_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
    assert!(published_event.is_ok(), "Expected a UserCommandCanonicityUpdate event to be published.");

    if let Ok(Ok(event)) = published_event {
        let published_payload: UserCommandCanonicityPayload = sonic_rs::from_str(&event.payload).unwrap();
        assert_eq!(published_payload.height, user_command_payload.height);
        assert_eq!(published_payload.state_hash, user_command_payload.state_hash);
        assert_eq!(published_payload.timestamp, user_command_payload.timestamp);
        assert_eq!(published_payload.txn_type, user_command_payload.txn_type);
        assert_eq!(published_payload.status, user_command_payload.status);
        assert_eq!(published_payload.sender, user_command_payload.sender);
        assert_eq!(published_payload.receiver, user_command_payload.receiver);
        assert_eq!(published_payload.nonce, user_command_payload.nonce);
        assert_eq!(published_payload.fee_nanomina, user_command_payload.fee_nanomina);
        assert_eq!(published_payload.amount_nanomina, user_command_payload.amount_nanomina);
        assert!(published_payload.canonical);
    }

    // Verify that events_published has been incremented
    assert_eq!(actor.events_published.load(Ordering::SeqCst), 1);

    Ok(())
}

#[tokio::test]
async fn test_user_command_canonicity_actor_prunes_user_commands_on_transition_frontier() -> anyhow::Result<()> {
    use crate::stream::{mainnet_block_models::CommandStatus, payloads::UserCommandSummaryPayload};

    // Set up the actor
    let shared_publisher = Arc::new(SharedPublisher::new(200));
    let actor = UserCommandCanonicityActor::new(Arc::clone(&shared_publisher));

    // Insert user command summaries with different heights
    {
        let mut user_commands = actor.user_commands.lock().await;
        user_commands.insert(
            Height(5),
            vec![UserCommandSummaryPayload {
                height: 5,
                state_hash: "hash_5".to_string(),
                timestamp: 1000,
                txn_type: crate::stream::mainnet_block_models::CommandType::StakeDelegation,
                status: CommandStatus::Applied,
                sender: "sender_5".to_string(),
                receiver: "receiver_5".to_string(),
                nonce: 1,
                fee_nanomina: 5000,
                amount_nanomina: 10000,
                ..Default::default()
            }],
        );
        user_commands.insert(
            Height(10),
            vec![UserCommandSummaryPayload {
                height: 10,
                state_hash: "hash_10".to_string(),
                timestamp: 2000,
                txn_type: crate::stream::mainnet_block_models::CommandType::StakeDelegation,
                status: CommandStatus::Applied,
                sender: "sender_10".to_string(),
                receiver: "receiver_10".to_string(),
                nonce: 2,
                fee_nanomina: 1000,
                amount_nanomina: 20000,
                ..Default::default()
            }],
        );
    }

    // Trigger a TransitionFrontier event with height = 7 to prune user commands with height <= 7
    let transition_event = Event {
        event_type: EventType::TransitionFrontier,
        payload: sonic_rs::to_string(&7u64).unwrap(),
    };
    actor.handle_event(transition_event).await;

    // Verify that user commands with height <= 7 were removed
    {
        let user_commands = actor.user_commands.lock().await;
        assert!(!user_commands.contains_key(&Height(5)), "UserCommand with height 5 should have been pruned");
        assert!(
            user_commands.contains_key(&Height(10)),
            "UserCommand with height 10 should not have been pruned"
        );
    }

    Ok(())
}
