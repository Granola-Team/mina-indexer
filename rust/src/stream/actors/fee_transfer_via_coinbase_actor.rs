use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::stream::payloads::{
    AccountingEntry, AccountingEntryAccountType, AccountingEntryType, DoubleEntryRecordPayload, InternalCommandPayload, InternalCommandType,
    MainnetBlockPayload,
};
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct FeeTransferViaCoinbaseActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
}

impl FeeTransferViaCoinbaseActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "FeeTransferViaCoinbaseActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
        }
    }

    fn publish_coinbase_accounting_entry(&self, block_payload: &MainnetBlockPayload, fee: u64) {
        let payload = DoubleEntryRecordPayload {
            height: block_payload.height,
            lhs: vec![AccountingEntry {
                entry_type: AccountingEntryType::Debit,
                account: block_payload.coinbase_receiver.clone(),
                account_type: AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: fee,
                timestamp: block_payload.timestamp,
            }],
            rhs: vec![AccountingEntry {
                entry_type: AccountingEntryType::Credit,
                account: format!("BlockRewardPool#{}", block_payload.state_hash),
                account_type: AccountingEntryAccountType::VirtualAddess,
                amount_nanomina: fee,
                timestamp: block_payload.timestamp,
            }],
        };
        self.publish(Event {
            event_type: EventType::DoubleEntryTransaction,
            payload: sonic_rs::to_string(&payload).unwrap(),
        });
    }

    fn publish_fee_transfer_via_coinbase(&self, block_payload: &MainnetBlockPayload, fee: u64, receiver: String) {
        let payload = InternalCommandPayload {
            internal_command_type: InternalCommandType::FeeTransferViaCoinbase,
            height: block_payload.height,
            state_hash: block_payload.state_hash.to_string(),
            timestamp: block_payload.timestamp,
            recipient: receiver,
            amount_nanomina: fee,
        };
        self.publish(Event {
            event_type: EventType::InternalCommand,
            payload: sonic_rs::to_string(&payload).unwrap(),
        });
    }
}

#[async_trait]
impl Actor for FeeTransferViaCoinbaseActor {
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
                if let Some(fee_transfers_via_coinbase) = &block_payload.fee_transfer_via_coinbase {
                    for fee_transfer_via_coinbase in fee_transfers_via_coinbase.iter() {
                        let fee_transfer_amount = (fee_transfer_via_coinbase.fee * 1_000_000_000f64) as u64;
                        self.publish_coinbase_accounting_entry(&block_payload, fee_transfer_amount);
                        self.publish_fee_transfer_via_coinbase(&block_payload, fee_transfer_amount, fee_transfer_via_coinbase.receiver.to_string());
                    }
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
async fn test_handle_mainnet_block_event_publishes_fee_transfer_via_coinbase_event() {
    use super::*;
    use crate::stream::{
        events::{Event, EventType},
        mainnet_block_models::FeeTransferViaCoinbase,
        payloads::MainnetBlockPayload,
    };
    use std::sync::Arc;

    let shared_publisher = Arc::new(SharedPublisher::new(100));
    let actor = FeeTransferViaCoinbaseActor::new(Arc::clone(&shared_publisher));

    let block_payload = MainnetBlockPayload {
        height: 10,
        state_hash: "state_hash_example".to_string(),
        timestamp: 123456789,
        fee_transfer_via_coinbase: Some(vec![FeeTransferViaCoinbase {
            receiver: "receiver_example".to_string(),
            fee: 0.00005,
        }]),
        ..Default::default()
    };

    let payload_json = sonic_rs::to_string(&block_payload).unwrap();
    let event = Event {
        event_type: EventType::MainnetBlock,
        payload: payload_json,
    };

    let mut receiver = shared_publisher.subscribe();

    actor.handle_event(event).await;

    // Verify coinbase accounting entry
    if let Ok(received_event) = receiver.recv().await {
        assert_eq!(received_event.event_type, EventType::DoubleEntryTransaction);

        let accounting_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&received_event.payload).unwrap();

        // Check `lhs`
        assert_eq!(accounting_payload.height, block_payload.height);
        assert_eq!(accounting_payload.lhs.len(), 1);
        assert_eq!(accounting_payload.lhs[0].account, block_payload.coinbase_receiver);
        assert_eq!(accounting_payload.lhs[0].amount_nanomina, 50_000);
        assert_eq!(accounting_payload.lhs[0].entry_type, AccountingEntryType::Debit);

        // Check `rhs`
        assert_eq!(accounting_payload.rhs.len(), 1);
        assert_eq!(accounting_payload.rhs[0].account, format!("BlockRewardPool#{}", block_payload.state_hash));
        assert_eq!(accounting_payload.rhs[0].amount_nanomina, 50_000);
        assert_eq!(accounting_payload.rhs[0].entry_type, AccountingEntryType::Credit);
    }

    // Verify fee transfer via coinbase
    if let Ok(received_event) = receiver.recv().await {
        assert_eq!(received_event.event_type, EventType::InternalCommand);

        let fee_transfer_payload: InternalCommandPayload = sonic_rs::from_str(&received_event.payload).unwrap();
        assert_eq!(fee_transfer_payload.height, block_payload.height);
        assert_eq!(fee_transfer_payload.state_hash, block_payload.state_hash);
        assert_eq!(fee_transfer_payload.timestamp, block_payload.timestamp);
        assert_eq!(fee_transfer_payload.recipient, "receiver_example");
        assert_eq!(fee_transfer_payload.amount_nanomina, 50_000);
    }

    assert_eq!(actor.actor_outputs().load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_handle_mainnet_block_event_without_fee_transfer_via_coinbase() {
    use super::*;
    use crate::stream::{
        events::{Event, EventType},
        payloads::MainnetBlockPayload,
    };
    use std::sync::Arc;

    // Setup a shared publisher to capture published events
    let shared_publisher = Arc::new(SharedPublisher::new(100));
    let actor = FeeTransferViaCoinbaseActor::new(Arc::clone(&shared_publisher));

    // Create a MainnetBlockPayload without a FeeTransferViaCoinbase
    let block_payload = MainnetBlockPayload {
        height: 10,
        state_hash: "state_hash_example".to_string(),
        timestamp: 123456789,
        fee_transfer_via_coinbase: None,
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

    // Verify that no events were published
    assert!(receiver.try_recv().is_err());
    assert_eq!(actor.actor_outputs().load(Ordering::SeqCst), 0);
}
