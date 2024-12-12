use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::event_sourcing::payloads::{BerkeleyBlockPayload, ZkappCommandLogPayload};
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct ZkappCommandActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
}

impl ZkappCommandActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "ZkappCommandActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Actor for ZkappCommandActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.events_published
    }

    async fn handle_event(&self, event: Event) {
        if let EventType::BerkeleyBlock = event.event_type {
            let block_payload: BerkeleyBlockPayload = sonic_rs::from_str(&event.payload).unwrap();

            for zkapp_command in block_payload.zk_app_commands.iter() {
                let payload = ZkappCommandLogPayload {
                    height: block_payload.height,
                    state_hash: block_payload.state_hash.clone(),
                    txn_hash: zkapp_command.txn_hash(),
                    timestamp: block_payload.timestamp,
                    txn_type: zkapp_command.txn_type.clone(),
                    status: zkapp_command.status.clone(),
                    nonce: zkapp_command.nonce,
                    fee_nanomina: zkapp_command.fee_nanomina,
                    fee_payer: zkapp_command.fee_payer.clone(),
                    global_slot: block_payload.global_slot_since_genesis,
                    memo: zkapp_command.memo.clone(),
                };
                self.publish(Event {
                    event_type: EventType::ZkAppCommandLog,
                    payload: sonic_rs::to_string(&payload).unwrap(),
                });
            }
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[cfg(test)]
mod zkapp_command_actor_tests {
    use super::*;
    use crate::event_sourcing::{
        events::{Event, EventType},
        models::{CommandStatus, CommandType, ZkAppCommandSummary},
        payloads::BerkeleyBlockPayload,
    };
    use std::sync::{atomic::Ordering, Arc};

    #[tokio::test]
    async fn test_berkeley_block_actor_handle_event() {
        // Create a shared publisher to capture published events
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = ZkappCommandActor::new(Arc::clone(&shared_publisher));

        // Mock a BerkeleyBlockPayload with zkapp commands
        let zk_app_commands = vec![
            ZkAppCommandSummary {
                memo: "memo_1".to_string(),
                fee_payer: "fee_payer_1".to_string(),
                status: CommandStatus::Applied,
                txn_type: CommandType::Payment,
                nonce: 1,
                fee_nanomina: 1_000_000,
                account_updates: 2,
            },
            ZkAppCommandSummary {
                memo: "memo_2".to_string(),
                fee_payer: "fee_payer_2".to_string(),
                status: CommandStatus::Failed,
                txn_type: CommandType::StakeDelegation,
                nonce: 2,
                fee_nanomina: 2_000_000,
                account_updates: 3,
            },
        ];

        let block_payload = BerkeleyBlockPayload {
            height: 15,
            state_hash: "state_hash_berkeley".to_string(),
            global_slot_since_genesis: 25,
            zk_app_commands,
            timestamp: 1672531200,
            ..Default::default()
        };

        // Serialize the BerkeleyBlockPayload to JSON for the event payload
        let payload_json = sonic_rs::to_string(&block_payload).unwrap();
        let event = Event {
            event_type: EventType::BerkeleyBlock,
            payload: payload_json,
        };

        // Subscribe to the shared publisher to capture published events
        let mut receiver = shared_publisher.subscribe();

        // Call handle_event to process the BerkeleyBlock event
        actor.handle_event(event).await;

        // Capture and verify the published ZkappCommandLog events
        for expected_summary in block_payload.zk_app_commands.iter() {
            if let Ok(received_event) = receiver.recv().await {
                assert_eq!(received_event.event_type, EventType::ZkAppCommandLog);

                // Deserialize the payload of the ZkappCommandLog event
                let log_payload: ZkappCommandLogPayload = sonic_rs::from_str(&received_event.payload).unwrap();

                // Verify that the ZkappCommandLogPayload matches the expected values
                assert_eq!(log_payload.height, block_payload.height);
                assert_eq!(log_payload.state_hash, block_payload.state_hash);
                assert_eq!(log_payload.timestamp, block_payload.timestamp);
                assert_eq!(log_payload.fee_payer, expected_summary.fee_payer);
                assert_eq!(log_payload.memo, expected_summary.memo);
            } else {
                panic!("Did not receive expected ZkappCommandLog event from ZkappCommandActor.");
            }
        }

        // Verify that the event count matches the number of zkapp commands processed
        assert_eq!(actor.actor_outputs().load(Ordering::SeqCst), block_payload.zk_app_commands.len());
    }
}
