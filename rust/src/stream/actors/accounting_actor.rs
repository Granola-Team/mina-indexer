use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::stream::{
    mainnet_block_models::{CommandStatus, CommandType},
    payloads::{
        AccountingEntry, AccountingEntryAccountType, AccountingEntryType, DoubleEntryRecordPayload, InternalCommandCanonicityPayload, InternalCommandType,
        UserCommandCanonicityPayload,
    },
};
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct AccountingActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub entries_processed: AtomicUsize,
}

impl AccountingActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "AccountingActor".to_string(),
            shared_publisher,
            entries_processed: AtomicUsize::new(0),
        }
    }

    async fn publish_transaction(&self, record: &DoubleEntryRecordPayload) {
        record.verify();
        let event = Event {
            event_type: EventType::DoubleEntryTransaction,
            payload: sonic_rs::to_string(record).unwrap(),
        };

        self.shared_publisher.publish(event);
        self.entries_processed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    async fn process_internal_command(&self, payload: &InternalCommandCanonicityPayload) {
        let mut source_entry = AccountingEntry {
            entry_type: AccountingEntryType::Debit,
            account: match payload.internal_command_type {
                InternalCommandType::Coinbase => format!("MinaCoinbasePayment#{}", payload.state_hash),
                InternalCommandType::FeeTransfer | InternalCommandType::FeeTransferViaCoinbase => format!("BlockRewardPool#{}", payload.state_hash),
            },
            account_type: AccountingEntryAccountType::VirtualAddess,
            amount_nanomina: payload.amount_nanomina,
            timestamp: payload.timestamp,
        };
        let mut recipient_entry = AccountingEntry {
            entry_type: AccountingEntryType::Credit,
            account: payload.recipient.clone(),
            account_type: AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: payload.amount_nanomina,
            timestamp: payload.timestamp,
        };

        if !payload.canonical {
            // Swap debits and credits for non-canonical entries
            source_entry.entry_type = AccountingEntryType::Credit;
            recipient_entry.entry_type = AccountingEntryType::Debit;
        }

        let double_entry_record = DoubleEntryRecordPayload {
            height: payload.height,
            lhs: vec![source_entry],
            rhs: vec![recipient_entry],
        };

        self.publish_transaction(&double_entry_record).await;
    }

    async fn process_user_command(&self, payload: &UserCommandCanonicityPayload) {
        let mut sender_entry = AccountingEntry {
            entry_type: AccountingEntryType::Debit,
            account: payload.sender.to_string(),
            account_type: AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: payload.amount_nanomina,
            timestamp: payload.timestamp,
        };
        let mut fee_payer_entry = AccountingEntry {
            entry_type: AccountingEntryType::Debit,
            account: payload.fee_payer.to_string(),
            account_type: AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: payload.fee_nanomina,
            timestamp: payload.timestamp,
        };
        let mut receiver_entry = AccountingEntry {
            entry_type: AccountingEntryType::Credit,
            account: payload.receiver.to_string(),
            account_type: AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: payload.amount_nanomina,
            timestamp: payload.timestamp,
        };
        let mut block_reward_pool_entry = AccountingEntry {
            entry_type: AccountingEntryType::Credit,
            account: format!("BlockRewardPool#{}", payload.state_hash),
            account_type: AccountingEntryAccountType::VirtualAddess,
            amount_nanomina: payload.fee_nanomina,
            timestamp: payload.timestamp,
        };

        if !payload.canonical {
            // swap debits and credits
            sender_entry.entry_type = AccountingEntryType::Credit;
            fee_payer_entry.entry_type = AccountingEntryType::Credit;
            receiver_entry.entry_type = AccountingEntryType::Debit;
            block_reward_pool_entry.entry_type = AccountingEntryType::Debit;
        }
        if payload.status == CommandStatus::Failed {
            // no balance is transferred but fees are paid
            sender_entry.amount_nanomina = 0;
            receiver_entry.amount_nanomina = 0;
        }
        let (lhs, rhs) = match payload.txn_type {
            // stake delegation does not affect balance of sender or receiver from accounting perspective
            CommandType::StakeDelegation => (vec![fee_payer_entry], vec![block_reward_pool_entry]),
            CommandType::Payment => (vec![sender_entry, fee_payer_entry], vec![receiver_entry, block_reward_pool_entry]),
        };

        let double_entry_record = DoubleEntryRecordPayload {
            height: payload.height,
            lhs,
            rhs,
        };

        self.publish_transaction(&double_entry_record).await;
    }
}

#[async_trait]
impl Actor for AccountingActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.entries_processed
    }

    async fn handle_event(&self, event: Event) {
        match event.event_type {
            EventType::InternalCommandCanonicityUpdate => {
                let payload: InternalCommandCanonicityPayload = sonic_rs::from_str(&event.payload).unwrap();
                // not canonical, and never wasn't before. No need to deduct
                if !payload.canonical && !payload.was_canonical {
                    return;
                }
                self.process_internal_command(&payload).await;
            }
            EventType::UserCommandCanonicityUpdate => {
                let payload: UserCommandCanonicityPayload = sonic_rs::from_str(&event.payload).unwrap();
                // not canonical, and never wasn't before. No need to deduct
                if !payload.canonical && !payload.was_canonical {
                    return;
                }
                self.process_user_command(&payload).await;
            }
            _ => {}
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[cfg(test)]
mod accounting_actor_tests {
    use super::*;
    use crate::stream::payloads::{DoubleEntryRecordPayload, InternalCommandCanonicityPayload, InternalCommandType, UserCommandCanonicityPayload};
    use std::sync::{atomic::Ordering, Arc};
    use tokio::time::timeout;

    // Helper function to set up actor and subscriber
    fn setup_actor() -> (Arc<AccountingActor>, tokio::sync::broadcast::Receiver<Event>) {
        let shared_publisher = Arc::new(SharedPublisher::new(200));
        let actor = Arc::new(AccountingActor::new(Arc::clone(&shared_publisher)));
        let receiver = shared_publisher.subscribe();
        (actor, receiver)
    }

    #[tokio::test]
    async fn test_process_user_command_canonical_true_but_failed_with_fee_paid() {
        let (actor, mut receiver) = setup_actor();

        let payload = UserCommandCanonicityPayload {
            height: 200,
            state_hash: "state_hash_3".to_string(),
            timestamp: 1620000200,
            txn_type: crate::stream::mainnet_block_models::CommandType::Payment,
            status: CommandStatus::Failed,
            sender: "B62qsender1".to_string(),
            receiver: "B62qreceiver1".to_string(),
            fee_payer: "B62qsender1".to_string(),
            nonce: 1,
            fee_nanomina: 1_000_000,
            amount_nanomina: 100_000_000,
            canonical: true,
            was_canonical: false,
        };

        actor.process_user_command(&payload).await;
        let published_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(published_event.is_ok(), "Expected a DoubleEntryTransaction event to be published.");

        if let Ok(Ok(event)) = published_event {
            let published_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(published_payload.height, payload.height);
            assert_eq!(published_payload.lhs.len(), 2);
            assert_eq!(published_payload.rhs.len(), 2);

            assert_eq!(published_payload.lhs[0].entry_type, AccountingEntryType::Debit);
            assert_eq!(published_payload.lhs[0].account, payload.sender);
            assert_eq!(published_payload.rhs[0].entry_type, AccountingEntryType::Credit);
            assert_eq!(published_payload.rhs[0].account, payload.receiver);

            // no money sent but fee still paid
            assert_eq!(published_payload.rhs[0].amount_nanomina, 0);
            assert_eq!(published_payload.lhs[0].amount_nanomina, 0);
            assert_eq!(published_payload.rhs[1].amount_nanomina, 1_000_000);
            assert_eq!(published_payload.lhs[1].amount_nanomina, 1_000_000);
        }

        assert_eq!(actor.entries_processed.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_process_user_command_canonical_true() {
        let (actor, mut receiver) = setup_actor();

        let payload = UserCommandCanonicityPayload {
            height: 200,
            state_hash: "state_hash_3".to_string(),
            timestamp: 1620000200,
            txn_type: crate::stream::mainnet_block_models::CommandType::Payment,
            status: CommandStatus::Applied,
            sender: "B62qsender1".to_string(),
            receiver: "B62qreceiver1".to_string(),
            fee_payer: "B62qsender1".to_string(),
            nonce: 1,
            fee_nanomina: 1_000_000,
            amount_nanomina: 100_000_000,
            canonical: true,
            was_canonical: false,
        };

        actor.process_user_command(&payload).await;
        let published_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(published_event.is_ok(), "Expected a DoubleEntryTransaction event to be published.");

        if let Ok(Ok(event)) = published_event {
            let published_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(published_payload.height, payload.height);
            assert_eq!(published_payload.lhs.len(), 2);
            assert_eq!(published_payload.rhs.len(), 2);

            assert_eq!(published_payload.lhs[0].entry_type, AccountingEntryType::Debit);
            assert_eq!(published_payload.lhs[0].account, payload.sender);
            assert_eq!(published_payload.rhs[0].entry_type, AccountingEntryType::Credit);
            assert_eq!(published_payload.rhs[0].account, payload.receiver);
        }

        assert_eq!(actor.entries_processed.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_process_user_command_canonical_false() {
        let (actor, mut receiver) = setup_actor();

        let payload = UserCommandCanonicityPayload {
            height: 200,
            state_hash: "state_hash_3".to_string(),
            timestamp: 1620000200,
            txn_type: crate::stream::mainnet_block_models::CommandType::Payment,
            status: CommandStatus::Applied,
            sender: "B62qsender1".to_string(),
            receiver: "B62qreceiver1".to_string(),
            fee_payer: "B62qsender1".to_string(),
            nonce: 1,
            fee_nanomina: 1_000_000,
            amount_nanomina: 100_000_000,
            canonical: false,
            was_canonical: false,
        };

        actor
            .handle_event(Event {
                event_type: EventType::UserCommandCanonicityUpdate,
                payload: sonic_rs::to_string(&payload).unwrap(),
            })
            .await;
        let published_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(published_event.is_err(), "Did not expected a DoubleEntryTransaction event to be published.");

        assert_eq!(actor.entries_processed.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_process_internal_command_coinbase_canonical_true() {
        let (actor, mut receiver) = setup_actor();

        let payload = InternalCommandCanonicityPayload {
            internal_command_type: InternalCommandType::Coinbase,
            height: 300,
            state_hash: "state_hash_4".to_string(),
            timestamp: 1620000300,
            amount_nanomina: 200_000_000,
            recipient: "B62qrecipient1".to_string(),
            canonical: true,
            was_canonical: false,
        };

        actor.process_internal_command(&payload).await;
        let published_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(published_event.is_ok(), "Expected a DoubleEntryTransaction event to be published.");

        if let Ok(Ok(event)) = published_event {
            let published_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(published_payload.height, payload.height);
            assert_eq!(published_payload.lhs[0].entry_type, AccountingEntryType::Debit);
            assert_eq!(published_payload.lhs[0].account, format!("MinaCoinbasePayment#{}", payload.state_hash));
            assert_eq!(published_payload.rhs[0].entry_type, AccountingEntryType::Credit);
            assert_eq!(published_payload.rhs[0].account, payload.recipient);
        }
    }

    #[tokio::test]
    async fn test_process_internal_command_coinbase_canonical_false() {
        let (actor, mut receiver) = setup_actor();

        let payload = InternalCommandCanonicityPayload {
            internal_command_type: InternalCommandType::Coinbase,
            height: 300,
            state_hash: "state_hash_4".to_string(),
            timestamp: 1620000300,
            amount_nanomina: 200_000_000,
            recipient: "B62qrecipient1".to_string(),
            canonical: false,
            was_canonical: true,
        };

        actor.process_internal_command(&payload).await;
        let published_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(published_event.is_ok(), "Expected a DoubleEntryTransaction event to be published.");

        if let Ok(Ok(event)) = published_event {
            let published_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(published_payload.height, payload.height);
            assert_eq!(published_payload.lhs[0].entry_type, AccountingEntryType::Credit);
            assert_eq!(published_payload.lhs[0].account, format!("MinaCoinbasePayment#{}", payload.state_hash));
            assert_eq!(published_payload.rhs[0].entry_type, AccountingEntryType::Debit);
            assert_eq!(published_payload.rhs[0].account, payload.recipient);
        }
    }

    #[tokio::test]
    async fn test_process_internal_command_fee_transfer_canonical_true() {
        let (actor, mut receiver) = setup_actor();

        let payload = InternalCommandCanonicityPayload {
            internal_command_type: InternalCommandType::FeeTransfer,
            height: 300,
            state_hash: "state_hash_5".to_string(),
            timestamp: 1620000400,
            amount_nanomina: 50_000_000,
            recipient: "B62qrecipient2".to_string(),
            canonical: true,
            was_canonical: false,
        };

        actor.process_internal_command(&payload).await;
        let published_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(published_event.is_ok(), "Expected a DoubleEntryTransaction event to be published.");

        if let Ok(Ok(event)) = published_event {
            let published_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(published_payload.height, payload.height);
            assert_eq!(published_payload.lhs[0].entry_type, AccountingEntryType::Debit);
            assert_eq!(published_payload.lhs[0].account, format!("BlockRewardPool#{}", payload.state_hash));
            assert_eq!(published_payload.rhs[0].entry_type, AccountingEntryType::Credit);
            assert_eq!(published_payload.rhs[0].account, payload.recipient);
        }
    }

    #[tokio::test]
    async fn test_process_internal_command_fee_transfer_canonical_false() {
        let (actor, mut receiver) = setup_actor();

        let payload = InternalCommandCanonicityPayload {
            internal_command_type: InternalCommandType::FeeTransfer,
            height: 300,
            state_hash: "state_hash_5".to_string(),
            timestamp: 1620000400,
            amount_nanomina: 50_000_000,
            recipient: "B62qrecipient2".to_string(),
            canonical: false,
            was_canonical: false,
        };

        actor
            .handle_event(Event {
                event_type: EventType::InternalCommandCanonicityUpdate,
                payload: sonic_rs::to_string(&payload).unwrap(),
            })
            .await;
        let published_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(published_event.is_err(), "Did not expected a DoubleEntryTransaction event to be published.");

        assert_eq!(actor.entries_processed.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_process_internal_command_fee_transfer_via_coinbase_canonical_true() {
        let (actor, mut receiver) = setup_actor();

        let payload = InternalCommandCanonicityPayload {
            internal_command_type: InternalCommandType::FeeTransferViaCoinbase,
            height: 300,
            state_hash: "state_hash_6".to_string(),
            timestamp: 1620000500,
            amount_nanomina: 75_000_000,
            recipient: "B62qrecipient3".to_string(),
            canonical: true,
            was_canonical: false,
        };

        actor.process_internal_command(&payload).await;
        let published_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(published_event.is_ok(), "Expected a DoubleEntryTransaction event to be published.");

        if let Ok(Ok(event)) = published_event {
            let published_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(published_payload.height, payload.height);
            assert_eq!(published_payload.lhs[0].entry_type, AccountingEntryType::Debit);
            assert_eq!(published_payload.lhs[0].account, format!("BlockRewardPool#{}", payload.state_hash));
            assert_eq!(published_payload.rhs[0].entry_type, AccountingEntryType::Credit);
            assert_eq!(published_payload.rhs[0].account, payload.recipient);
        }
    }

    #[tokio::test]
    async fn test_process_internal_command_fee_transfer_via_coinbase_canonical_false() {
        let (actor, mut receiver) = setup_actor();

        let payload = InternalCommandCanonicityPayload {
            internal_command_type: InternalCommandType::FeeTransferViaCoinbase,
            height: 300,
            state_hash: "state_hash_6".to_string(),
            timestamp: 1620000500,
            amount_nanomina: 75_000_000,
            recipient: "B62qrecipient3".to_string(),
            canonical: false,
            was_canonical: true,
        };

        actor.process_internal_command(&payload).await;
        let published_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(published_event.is_ok(), "Expected a DoubleEntryTransaction event to be published.");

        if let Ok(Ok(event)) = published_event {
            let published_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(published_payload.height, payload.height);
            assert_eq!(published_payload.lhs[0].entry_type, AccountingEntryType::Credit);
            assert_eq!(published_payload.lhs[0].account, format!("BlockRewardPool#{}", payload.state_hash));
            assert_eq!(published_payload.rhs[0].entry_type, AccountingEntryType::Debit);
            assert_eq!(published_payload.rhs[0].account, payload.recipient);
        }
    }
}
