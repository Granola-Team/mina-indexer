use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::event_sourcing::payloads::{MainnetBlockPayload, UserCommandLogPayload};
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct UserCommandLogActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
}

impl UserCommandLogActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "UserCommandActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Actor for UserCommandLogActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.events_published
    }
    async fn handle_event(&self, event: Event) {
        match event.event_type {
            EventType::MainnetBlock => {
                let block_payload: MainnetBlockPayload = sonic_rs::from_str(&event.payload).unwrap();
                for user_command in block_payload.user_commands.iter() {
                    let payload = UserCommandLogPayload {
                        height: block_payload.height,
                        global_slot: block_payload.global_slot,
                        txn_hash: user_command.txn_hash(),
                        state_hash: block_payload.state_hash.to_string(),
                        timestamp: block_payload.timestamp,
                        txn_type: user_command.txn_type.clone(),
                        status: user_command.status.clone(),
                        sender: user_command.sender.to_string(),
                        receiver: user_command.receiver.to_string(),
                        nonce: user_command.nonce,
                        fee_nanomina: user_command.fee_nanomina,
                        fee_payer: user_command.fee_payer.to_string(),
                        amount_nanomina: user_command.amount_nanomina,
                        memo: user_command.memo.to_string(),
                    };
                    self.publish(Event {
                        event_type: EventType::UserCommandLog,
                        payload: sonic_rs::to_string(&payload).unwrap(),
                    });
                }
            }
            EventType::BerkeleyBlock => {
                todo!("impl for berkeley block");
            }
            _ => {}
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[tokio::test]
async fn test_user_command_actor_handle_event() {
    use super::*;
    use crate::event_sourcing::{
        events::{Event, EventType},
        mainnet_block_models::*,
        payloads::{MainnetBlockPayload, UserCommandLogPayload},
    };
    use std::sync::Arc;

    // Create a shared publisher to capture published events
    let shared_publisher = Arc::new(SharedPublisher::new(100));
    let actor = UserCommandLogActor::new(Arc::clone(&shared_publisher));

    // Mock a MainnetBlockPayload with user commands
    let user_commands = vec![
        CommandSummary {
            memo: "memo_1".to_string(),
            fee_payer: "payer_1".to_string(),
            sender: "sender_1".to_string(),
            receiver: "receiver_1".to_string(),
            status: CommandStatus::Applied,
            txn_type: CommandType::Payment,
            nonce: 1,
            fee_nanomina: 10_000_000,
            amount_nanomina: 500_000_000,
        },
        CommandSummary {
            memo: "memo_2".to_string(),
            fee_payer: "payer_2".to_string(),
            sender: "sender_2".to_string(),
            receiver: "receiver_2".to_string(),
            status: CommandStatus::Failed,
            txn_type: CommandType::StakeDelegation,
            nonce: 2,
            fee_nanomina: 5_000_000,
            amount_nanomina: 0,
        },
    ];

    // MainnetBlockPayload with sample user commands
    let block_payload = MainnetBlockPayload {
        height: 10,
        state_hash: "state_hash".to_string(),
        previous_state_hash: "previous_state_hash".to_string(),
        last_vrf_output: "last_vrf_output".to_string(),
        user_command_count: 2,
        user_commands,
        snark_work_count: 0,
        snark_work: vec![],
        timestamp: 123414312431234,
        coinbase_receiver: "coinbase_receiver".to_string(),
        coinbase_reward_nanomina: 720_000_000_000,
        global_slot_since_genesis: 16,
        ..Default::default()
    };

    // Serialize the MainnetBlockPayload to JSON for the event payload
    let payload_json = sonic_rs::to_string(&block_payload).unwrap();
    let event = Event {
        event_type: EventType::MainnetBlock,
        payload: payload_json,
    };

    // Subscribe to the shared publisher to capture published events
    let mut receiver = shared_publisher.subscribe();

    // Call handle_event to process the MainnetBlock event
    actor.handle_event(event).await;

    // Capture and verify the published UserCommandSummary events
    for user_command in block_payload.user_commands.iter() {
        // Check if the UserCommandSummary event was published
        if let Ok(received_event) = receiver.recv().await {
            assert_eq!(received_event.event_type, EventType::UserCommandLog);

            // Deserialize the payload of the UserCommandSummary event
            let summary_payload: UserCommandLogPayload = sonic_rs::from_str(&received_event.payload).unwrap();

            // Verify that the UserCommandSummaryPayload matches the expected values
            assert_eq!(summary_payload.height, block_payload.height);
            assert_eq!(summary_payload.state_hash, block_payload.state_hash);
            assert_eq!(summary_payload.timestamp, block_payload.timestamp);
            assert_eq!(summary_payload.txn_type, user_command.txn_type);
            assert_eq!(summary_payload.status, user_command.status);
            assert_eq!(summary_payload.sender, user_command.sender);
            assert_eq!(summary_payload.receiver, user_command.receiver);
            assert_eq!(summary_payload.nonce, user_command.nonce);
            assert_eq!(summary_payload.fee_nanomina, user_command.fee_nanomina);
            assert_eq!(summary_payload.amount_nanomina, user_command.amount_nanomina);
        } else {
            panic!("Did not receive expected UserCommandSummary event from UserCommandActor.");
        }
    }

    // Verify that the event count matches the number of user commands processed
    assert_eq!(actor.actor_outputs().load(Ordering::SeqCst), block_payload.user_commands.len());
}
