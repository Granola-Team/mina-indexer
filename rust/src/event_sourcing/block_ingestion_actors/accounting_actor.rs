use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::event_sourcing::{
    models::{CommandStatus, CommandType},
    payloads::{
        AccountingEntry, AccountingEntryAccountType, AccountingEntryType, CanonicalBatchZkappCommandLogPayload, CanonicalInternalCommandLogPayload,
        CanonicalUserCommandLogPayload, DoubleEntryRecordPayload, InternalCommandType, LedgerDestination, NewAccountPayload,
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

    async fn process_internal_command(&self, payload: &CanonicalInternalCommandLogPayload) {
        if payload.internal_command_type == InternalCommandType::FeeTransferViaCoinbase {
            {
                let mut source = AccountingEntry {
                    transfer_type: "BlockRewardPool".to_string(),
                    counterparty: format!("BlockRewardPool#{}", payload.state_hash),
                    entry_type: AccountingEntryType::Debit,
                    account: payload.source.clone().unwrap(),
                    account_type: AccountingEntryAccountType::BlockchainAddress,
                    amount_nanomina: payload.amount_nanomina,
                    timestamp: payload.timestamp,
                };
                let mut recipient = AccountingEntry {
                    transfer_type: "BlockRewardPool".to_string(),
                    counterparty: payload.source.clone().unwrap(),
                    entry_type: AccountingEntryType::Credit,
                    account: format!("BlockRewardPool#{}", payload.state_hash),
                    account_type: AccountingEntryAccountType::VirtualAddess,
                    amount_nanomina: payload.amount_nanomina,
                    timestamp: payload.timestamp,
                };
                // Swap debits and credits for non-canonical entries
                if !payload.canonical {
                    source.entry_type = AccountingEntryType::Credit;
                    recipient.entry_type = AccountingEntryType::Debit;
                }
                let double_entry_record = DoubleEntryRecordPayload {
                    height: payload.height,
                    ledger_destination: LedgerDestination::BlockchainLedger,
                    state_hash: payload.state_hash.to_string(),
                    lhs: vec![source],
                    rhs: vec![recipient],
                };

                self.publish_transaction(&double_entry_record).await;
            }
            {
                let mut source = AccountingEntry {
                    transfer_type: InternalCommandType::FeeTransferViaCoinbase.to_string(),
                    counterparty: payload.recipient.to_string(),
                    entry_type: AccountingEntryType::Debit,
                    account: format!("BlockRewardPool#{}", payload.state_hash),
                    account_type: AccountingEntryAccountType::VirtualAddess,
                    amount_nanomina: payload.amount_nanomina,
                    timestamp: payload.timestamp,
                };
                let mut recipient = AccountingEntry {
                    transfer_type: InternalCommandType::FeeTransferViaCoinbase.to_string(),
                    counterparty: format!("BlockRewardPool#{}", payload.state_hash),
                    entry_type: AccountingEntryType::Credit,
                    account: payload.recipient.to_string(),
                    account_type: AccountingEntryAccountType::BlockchainAddress,
                    amount_nanomina: payload.amount_nanomina,
                    timestamp: payload.timestamp,
                };
                // Swap debits and credits for non-canonical entries
                if !payload.canonical {
                    source.entry_type = AccountingEntryType::Credit;
                    recipient.entry_type = AccountingEntryType::Debit;
                }
                let double_entry_record = DoubleEntryRecordPayload {
                    height: payload.height,
                    state_hash: payload.state_hash.to_string(),
                    ledger_destination: LedgerDestination::BlockchainLedger,
                    lhs: vec![source],
                    rhs: vec![recipient],
                };

                self.publish_transaction(&double_entry_record).await;
            }
        } else {
            let mut source = AccountingEntry {
                transfer_type: payload.internal_command_type.to_string(),
                counterparty: payload.recipient.to_string(),
                entry_type: AccountingEntryType::Debit,
                account: match payload.internal_command_type {
                    InternalCommandType::Coinbase => format!("MinaCoinbasePayment#{}", payload.state_hash),
                    _ => format!("BlockRewardPool#{}", payload.state_hash),
                },
                account_type: AccountingEntryAccountType::VirtualAddess,
                amount_nanomina: payload.amount_nanomina,
                timestamp: payload.timestamp,
            };
            let mut recipient = AccountingEntry {
                transfer_type: payload.internal_command_type.to_string(),
                counterparty: match payload.internal_command_type {
                    InternalCommandType::Coinbase => format!("MinaCoinbasePayment#{}", payload.state_hash),
                    _ => format!("BlockRewardPool#{}", payload.state_hash),
                },
                entry_type: AccountingEntryType::Credit,
                account: payload.recipient.clone(),
                account_type: AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: payload.amount_nanomina,
                timestamp: payload.timestamp,
            };

            if !payload.canonical {
                // Swap debits and credits for non-canonical entries
                source.entry_type = AccountingEntryType::Credit;
                recipient.entry_type = AccountingEntryType::Debit;
            }

            let double_entry_record = DoubleEntryRecordPayload {
                height: payload.height,
                state_hash: payload.state_hash.to_string(),
                ledger_destination: LedgerDestination::BlockchainLedger,
                lhs: vec![source],
                rhs: vec![recipient],
            };

            self.publish_transaction(&double_entry_record).await;
        }
    }

    async fn process_user_commands(&self, payload: &CanonicalUserCommandLogPayload) {
        let mut sender_entry = AccountingEntry {
            transfer_type: payload.txn_type.to_string(),
            counterparty: payload.receiver.to_string(),
            entry_type: AccountingEntryType::Debit,
            account: payload.sender.to_string(),
            account_type: AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: payload.amount_nanomina,
            timestamp: payload.timestamp,
        };
        let mut receiver_entry = AccountingEntry {
            transfer_type: payload.txn_type.to_string(),
            counterparty: payload.sender.to_string(),
            entry_type: AccountingEntryType::Credit,
            account: payload.receiver.to_string(),
            account_type: AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: payload.amount_nanomina,
            timestamp: payload.timestamp,
        };
        if !payload.canonical {
            // Swap debits and credits
            sender_entry.entry_type = AccountingEntryType::Credit;
            receiver_entry.entry_type = AccountingEntryType::Debit;
        }
        if payload.status == CommandStatus::Applied && payload.txn_type != CommandType::StakeDelegation {
            // Split into two separate transactions for publishing
            let txn_1 = DoubleEntryRecordPayload {
                height: payload.height,
                state_hash: payload.state_hash.to_string(),
                ledger_destination: LedgerDestination::BlockchainLedger,
                lhs: vec![sender_entry],   // Sender entry
                rhs: vec![receiver_entry], // Receiver entry
            };

            self.publish_transaction(&txn_1).await;
        }

        let mut fee_payer_entry = AccountingEntry {
            counterparty: format!("BlockRewardPool#{}", payload.state_hash),
            transfer_type: "BlockRewardPool".to_string(),
            entry_type: AccountingEntryType::Debit,
            account: payload.fee_payer.to_string(),
            account_type: AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: payload.fee_nanomina,
            timestamp: payload.timestamp,
        };

        let mut block_reward_pool_entry = AccountingEntry {
            counterparty: payload.fee_payer.to_string(),
            transfer_type: "BlockRewardPool".to_string(),
            entry_type: AccountingEntryType::Credit,
            account: format!("BlockRewardPool#{}", payload.state_hash),
            account_type: AccountingEntryAccountType::VirtualAddess,
            amount_nanomina: payload.fee_nanomina,
            timestamp: payload.timestamp,
        };

        if !payload.canonical {
            // Swap debits and credits
            fee_payer_entry.entry_type = AccountingEntryType::Credit;
            block_reward_pool_entry.entry_type = AccountingEntryType::Debit;
        }

        let txn_2 = DoubleEntryRecordPayload {
            height: payload.height,
            state_hash: payload.state_hash.to_string(),
            ledger_destination: LedgerDestination::BlockchainLedger,
            lhs: vec![fee_payer_entry],
            rhs: vec![block_reward_pool_entry],
        };

        self.publish_transaction(&txn_2).await;
    }

    async fn process_batch_zk_app_commands(&self, payload: &CanonicalBatchZkappCommandLogPayload) {
        for command in &payload.commands {
            let mut fee_payer_entry = AccountingEntry {
                counterparty: format!("BlockRewardPool#{}", payload.state_hash),
                transfer_type: "BlockRewardPool".to_string(),
                entry_type: AccountingEntryType::Debit,
                account: command.fee_payer.to_string(),
                account_type: AccountingEntryAccountType::BlockchainAddress,
                amount_nanomina: command.fee_nanomina,
                timestamp: payload.timestamp,
            };

            let mut block_reward_pool_entry = AccountingEntry {
                counterparty: command.fee_payer.to_string(),
                transfer_type: "BlockRewardPool".to_string(),
                entry_type: AccountingEntryType::Credit,
                account: format!("BlockRewardPool#{}", payload.state_hash),
                account_type: AccountingEntryAccountType::VirtualAddess,
                amount_nanomina: command.fee_nanomina,
                timestamp: payload.timestamp,
            };

            if !payload.canonical {
                // Swap debits and credits
                fee_payer_entry.entry_type = AccountingEntryType::Credit;
                block_reward_pool_entry.entry_type = AccountingEntryType::Debit;
            }

            let txn = DoubleEntryRecordPayload {
                height: payload.height,
                state_hash: payload.state_hash.to_string(),
                ledger_destination: LedgerDestination::BlockchainLedger,
                lhs: vec![fee_payer_entry],
                rhs: vec![block_reward_pool_entry],
            };

            self.publish_transaction(&txn).await;
        }
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
            EventType::NewAccount => {
                let payload: NewAccountPayload = sonic_rs::from_str(&event.payload).unwrap();
                if payload.height < 2 {
                    // genesis ledger accounts pay no account creation fees
                    // magic mina receiver in block 1 is also no subject to account creation fee
                    return;
                }
                let double_entry_record = DoubleEntryRecordPayload {
                    height: payload.height,
                    state_hash: payload.state_hash.to_string(),
                    ledger_destination: LedgerDestination::BlockchainLedger,
                    lhs: vec![AccountingEntry {
                        counterparty: format!("AccountCreationFee#{}", payload.state_hash),
                        transfer_type: "AccountCreationFee".to_string(),
                        entry_type: AccountingEntryType::Debit,
                        account: payload.account.to_string(),
                        account_type: AccountingEntryAccountType::BlockchainAddress,
                        amount_nanomina: 1_000_000_000,
                        timestamp: 0,
                    }],
                    rhs: vec![AccountingEntry {
                        counterparty: payload.account,
                        transfer_type: "AccountCreationFee".to_string(),
                        entry_type: AccountingEntryType::Credit,
                        account: format!("AccountCreationFee#{}", payload.state_hash),
                        account_type: AccountingEntryAccountType::VirtualAddess,
                        amount_nanomina: 1_000_000_000,
                        timestamp: 0,
                    }],
                };

                self.publish_transaction(&double_entry_record).await;
            }
            EventType::CanonicalInternalCommandLog => {
                let payload: CanonicalInternalCommandLogPayload = sonic_rs::from_str(&event.payload).unwrap();
                // not canonical, and wasn't before. No need to deduct
                if !payload.canonical && !payload.was_canonical {
                    return;
                }
                self.process_internal_command(&payload).await;
            }
            EventType::CanonicalUserCommandLog => {
                let payload: CanonicalUserCommandLogPayload = sonic_rs::from_str(&event.payload).unwrap();
                // not canonical, and wasn't before. No need to deduct
                if !payload.canonical && !payload.was_canonical {
                    return;
                }
                self.process_user_commands(&payload).await;
            }
            EventType::CanonicalBatchZkappCommandLog => {
                let payload: CanonicalBatchZkappCommandLogPayload = sonic_rs::from_str(&event.payload).unwrap();
                // not canonical, and wasn't before. No need to deduct
                if !payload.canonical && !payload.was_canonical {
                    return;
                }
                self.process_batch_zk_app_commands(&payload).await;
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
    use crate::event_sourcing::{
        models::ZkAppCommandSummary,
        payloads::{CanonicalInternalCommandLogPayload, CanonicalUserCommandLogPayload, DoubleEntryRecordPayload, InternalCommandType},
    };
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

        let payload = CanonicalUserCommandLogPayload {
            height: 200,
            global_slot: 0,
            txn_hash: "txn_hash".to_string(),
            state_hash: "state_hash_3".to_string(),
            timestamp: 1620000200,
            txn_type: crate::event_sourcing::models::CommandType::Payment,
            status: CommandStatus::Failed,
            sender: "B62qsender1".to_string(),
            receiver: "B62qreceiver1".to_string(),
            fee_payer: "B62qsender1".to_string(),
            nonce: 1,
            memo: "halp".to_string(),
            fee_nanomina: 1_000_000,
            amount_nanomina: 100_000_000,
            canonical: true,
            was_canonical: false,
        };

        actor.process_user_commands(&payload).await;

        // Verify the first published transaction (fee payer to block reward pool)
        let published_event_1 = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(published_event_1.is_ok(), "Expected the first DoubleEntryTransaction event to be published.");

        if let Ok(Ok(event)) = published_event_1 {
            let published_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(published_payload.height, payload.height);

            // Verify the fee transaction
            assert_eq!(published_payload.lhs.len(), 1);
            assert_eq!(published_payload.rhs.len(), 1);

            // Debit: Fee payer
            assert_eq!(published_payload.lhs[0].entry_type, AccountingEntryType::Debit);
            assert_eq!(published_payload.lhs[0].account, payload.fee_payer);
            assert_eq!(published_payload.lhs[0].amount_nanomina, payload.fee_nanomina);

            // Credit: Block reward pool
            assert_eq!(published_payload.rhs[0].entry_type, AccountingEntryType::Credit);
            assert_eq!(published_payload.rhs[0].account, format!("BlockRewardPool#{}", payload.state_hash));
            assert_eq!(published_payload.rhs[0].amount_nanomina, payload.fee_nanomina);
        }

        // Verify that no balance transfer transaction is published for a failed status
        let published_event_2 = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(
            published_event_2.is_err(),
            "No balance transfer transaction should be published for a failed status."
        );

        assert_eq!(actor.entries_processed.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_process_user_command_canonical_true() {
        let (actor, mut receiver) = setup_actor();

        let payload = CanonicalUserCommandLogPayload {
            height: 200,
            memo: "halp".to_string(),
            global_slot: 0,
            txn_hash: "txn_hash".to_string(),
            state_hash: "state_hash_3".to_string(),
            timestamp: 1620000200,
            txn_type: crate::event_sourcing::models::CommandType::Payment,
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

        actor.process_user_commands(&payload).await;

        // Verify the first published transaction (sender to receiver)
        let published_event_1 = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(published_event_1.is_ok(), "Expected the first DoubleEntryTransaction event to be published.");

        if let Ok(Ok(event)) = published_event_1 {
            let published_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(published_payload.height, payload.height);

            // Verify the balance transfer transaction
            assert_eq!(published_payload.lhs.len(), 1);
            assert_eq!(published_payload.rhs.len(), 1);

            assert_eq!(published_payload.lhs[0].entry_type, AccountingEntryType::Debit);
            assert_eq!(published_payload.lhs[0].account, payload.sender);
            assert_eq!(published_payload.lhs[0].amount_nanomina, payload.amount_nanomina);

            assert_eq!(published_payload.rhs[0].entry_type, AccountingEntryType::Credit);
            assert_eq!(published_payload.rhs[0].account, payload.receiver);
            assert_eq!(published_payload.rhs[0].amount_nanomina, payload.amount_nanomina);
        }

        // Verify the second published transaction (fee payer to block reward pool)
        let published_event_2 = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(published_event_2.is_ok(), "Expected the second DoubleEntryTransaction event to be published.");

        if let Ok(Ok(event)) = published_event_2 {
            let published_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(published_payload.height, payload.height);

            // Verify the fee transaction
            assert_eq!(published_payload.lhs.len(), 1);
            assert_eq!(published_payload.rhs.len(), 1);

            assert_eq!(published_payload.lhs[0].entry_type, AccountingEntryType::Debit);
            assert_eq!(published_payload.lhs[0].account, payload.fee_payer);
            assert_eq!(published_payload.lhs[0].amount_nanomina, payload.fee_nanomina);

            assert_eq!(published_payload.rhs[0].entry_type, AccountingEntryType::Credit);
            assert_eq!(published_payload.rhs[0].account, format!("BlockRewardPool#{}", payload.state_hash));
            assert_eq!(published_payload.rhs[0].amount_nanomina, payload.fee_nanomina);
        }

        assert_eq!(actor.entries_processed.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_process_user_command_canonical_false() {
        let (actor, mut receiver) = setup_actor();

        let payload = CanonicalUserCommandLogPayload {
            height: 200,
            global_slot: 0,
            memo: "halp".to_string(),
            txn_hash: "txn_hash".to_string(),
            state_hash: "state_hash_3".to_string(),
            timestamp: 1620000200,
            txn_type: crate::event_sourcing::models::CommandType::Payment,
            status: CommandStatus::Applied,
            sender: "B62qsender1".to_string(),
            receiver: "B62qreceiver1".to_string(),
            fee_payer: "B62qsender1".to_string(),
            nonce: 1,
            fee_nanomina: 1_000_000,
            amount_nanomina: 100_000_000,
            canonical: false,
            was_canonical: true,
        };

        actor.process_user_commands(&payload).await;

        // Verify the first published transaction (receiver to sender due to non-canonical status)
        let published_event_1 = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(published_event_1.is_ok(), "Expected the first DoubleEntryTransaction event to be published.");

        if let Ok(Ok(event)) = published_event_1 {
            let published_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(published_payload.height, payload.height);

            // Verify the reversed balance transfer transaction
            assert_eq!(published_payload.lhs.len(), 1);
            assert_eq!(published_payload.rhs.len(), 1);

            // Credit: Receiver
            assert_eq!(published_payload.lhs[0].entry_type, AccountingEntryType::Credit);
            assert_eq!(published_payload.lhs[0].account, payload.sender);
            assert_eq!(published_payload.lhs[0].amount_nanomina, payload.amount_nanomina);

            // Debit: Sender
            assert_eq!(published_payload.rhs[0].entry_type, AccountingEntryType::Debit);
            assert_eq!(published_payload.rhs[0].account, payload.receiver);
            assert_eq!(published_payload.rhs[0].amount_nanomina, payload.amount_nanomina);
        }

        // Verify the second published transaction (block reward pool to fee payer due to non-canonical status)
        let published_event_2 = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(published_event_2.is_ok(), "Expected the second DoubleEntryTransaction event to be published.");

        if let Ok(Ok(event)) = published_event_2 {
            let published_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(published_payload.height, payload.height);

            // Verify the reversed fee transaction
            assert_eq!(published_payload.lhs.len(), 1);
            assert_eq!(published_payload.rhs.len(), 1);

            // Debit: Block reward pool
            assert_eq!(published_payload.rhs[0].entry_type, AccountingEntryType::Debit);
            assert_eq!(published_payload.rhs[0].account, format!("BlockRewardPool#{}", payload.state_hash));
            assert_eq!(published_payload.rhs[0].amount_nanomina, payload.fee_nanomina);

            // Credit: Fee payer
            assert_eq!(published_payload.lhs[0].entry_type, AccountingEntryType::Credit);
            assert_eq!(published_payload.lhs[0].account, payload.fee_payer);
            assert_eq!(published_payload.lhs[0].amount_nanomina, payload.fee_nanomina);
        }

        assert_eq!(actor.entries_processed.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_process_internal_command_coinbase_canonical_true() {
        let (actor, mut receiver) = setup_actor();

        let payload = CanonicalInternalCommandLogPayload {
            internal_command_type: InternalCommandType::Coinbase,
            height: 300,
            state_hash: "state_hash_4".to_string(),
            timestamp: 1620000300,
            amount_nanomina: 200_000_000,
            recipient: "B62qrecipient1".to_string(),
            canonical: true,
            was_canonical: false,
            source: None,
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

        let payload = CanonicalInternalCommandLogPayload {
            internal_command_type: InternalCommandType::Coinbase,
            height: 300,
            state_hash: "state_hash_4".to_string(),
            timestamp: 1620000300,
            amount_nanomina: 200_000_000,
            recipient: "B62qrecipient1".to_string(),
            canonical: false,
            was_canonical: true,
            source: None,
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

        let payload = CanonicalInternalCommandLogPayload {
            internal_command_type: InternalCommandType::FeeTransfer,
            height: 300,
            state_hash: "state_hash_5".to_string(),
            timestamp: 1620000400,
            amount_nanomina: 50_000_000,
            recipient: "B62qrecipient2".to_string(),
            canonical: true,
            was_canonical: false,
            source: None,
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

        let payload = CanonicalInternalCommandLogPayload {
            internal_command_type: InternalCommandType::FeeTransfer,
            height: 300,
            state_hash: "state_hash_5".to_string(),
            timestamp: 1620000400,
            amount_nanomina: 50_000_000,
            recipient: "B62qrecipient2".to_string(),
            canonical: false,
            was_canonical: false,
            source: None,
        };

        actor
            .handle_event(Event {
                event_type: EventType::CanonicalInternalCommandLog,
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

        let payload = CanonicalInternalCommandLogPayload {
            internal_command_type: InternalCommandType::FeeTransferViaCoinbase,
            height: 300,
            state_hash: "state_hash_6".to_string(),
            timestamp: 1620000500,
            amount_nanomina: 75_000_000,
            recipient: "B62qrecipient3".to_string(),
            canonical: true,
            was_canonical: false,
            source: Some("coinbase_receiver".to_string()),
        };

        actor.process_internal_command(&payload).await;

        // Verify the first transaction (Coinbase Receiver -> BlockRewardPool)
        if let Ok(Ok(event)) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            let published_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(published_payload.height, payload.height);
            assert_eq!(published_payload.lhs[0].entry_type, AccountingEntryType::Debit);
            assert_eq!(published_payload.lhs[0].account, "coinbase_receiver".to_string());
            assert_eq!(published_payload.rhs[0].entry_type, AccountingEntryType::Credit);
            assert_eq!(published_payload.rhs[0].account, format!("BlockRewardPool#{}", payload.state_hash));
        } else {
            panic!("Expected the first DoubleEntryTransaction event.");
        }

        // Verify the second transaction (BlockRewardPool -> Recipient)
        if let Ok(Ok(event)) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            let published_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(published_payload.height, payload.height);
            assert_eq!(published_payload.lhs[0].entry_type, AccountingEntryType::Debit);
            assert_eq!(published_payload.lhs[0].account, format!("BlockRewardPool#{}", payload.state_hash));
            assert_eq!(published_payload.rhs[0].entry_type, AccountingEntryType::Credit);
            assert_eq!(published_payload.rhs[0].account, payload.recipient);
        } else {
            panic!("Expected the second DoubleEntryTransaction event.");
        }
    }

    #[tokio::test]
    async fn test_process_internal_command_fee_transfer_via_coinbase_canonical_false() {
        let (actor, mut receiver) = setup_actor();

        let payload = CanonicalInternalCommandLogPayload {
            internal_command_type: InternalCommandType::FeeTransferViaCoinbase,
            height: 300,
            state_hash: "state_hash_6".to_string(),
            timestamp: 1620000500,
            amount_nanomina: 75_000_000,
            recipient: "B62qrecipient3".to_string(),
            canonical: false,
            was_canonical: true,
            source: Some("coinbase_receiver".to_string()),
        };

        actor.process_internal_command(&payload).await;

        // Verify the first transaction (BlockRewardPool -> Coinbase Receiver)
        if let Ok(Ok(event)) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            let published_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(published_payload.height, payload.height);
            assert_eq!(published_payload.lhs[0].entry_type, AccountingEntryType::Credit);
            assert_eq!(published_payload.lhs[0].account, "coinbase_receiver".to_string());
            assert_eq!(published_payload.rhs[0].entry_type, AccountingEntryType::Debit);
            assert_eq!(published_payload.rhs[0].account, format!("BlockRewardPool#{}", payload.state_hash));
        } else {
            panic!("Expected the first DoubleEntryTransaction event.");
        }

        // Verify the second transaction (Recipient -> BlockRewardPool)
        if let Ok(Ok(event)) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            let published_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(published_payload.height, payload.height);
            assert_eq!(published_payload.lhs[0].entry_type, AccountingEntryType::Credit);
            assert_eq!(published_payload.lhs[0].account, format!("BlockRewardPool#{}", payload.state_hash));
            assert_eq!(published_payload.rhs[0].entry_type, AccountingEntryType::Debit);
            assert_eq!(published_payload.rhs[0].account, payload.recipient);
        } else {
            panic!("Expected the second DoubleEntryTransaction event.");
        }
    }

    #[tokio::test]
    async fn test_process_new_account_event() {
        let (actor, mut receiver) = setup_actor();

        // Mock NewAccountPayload
        let payload = NewAccountPayload {
            height: 100,
            state_hash: "3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4".to_string(),
            timestamp: 0,
            account: "B62qnewaccount1".to_string(),
            apply: true,
        };

        // Create the event
        let event = Event {
            event_type: EventType::NewAccount,
            payload: sonic_rs::to_string(&payload).unwrap(),
        };

        // Handle the NewAccount event
        actor.handle_event(event).await;

        // Verify the published DoubleEntryTransaction event
        let published_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(published_event.is_ok(), "Expected a DoubleEntryTransaction event to be published.");

        if let Ok(Ok(event)) = published_event {
            assert_eq!(event.event_type, EventType::DoubleEntryTransaction);

            // Deserialize and verify the DoubleEntryRecordPayload
            let published_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(published_payload.height, payload.height);

            assert_eq!(published_payload.lhs.len(), 1);
            assert_eq!(published_payload.rhs.len(), 1);

            // Verify the LHS (debit) entry
            assert_eq!(published_payload.lhs[0].entry_type, AccountingEntryType::Debit);
            assert_eq!(published_payload.lhs[0].account, payload.account);
            assert_eq!(published_payload.lhs[0].account_type, AccountingEntryAccountType::BlockchainAddress);
            assert_eq!(published_payload.lhs[0].amount_nanomina, 1_000_000_000);
            assert_eq!(published_payload.lhs[0].timestamp, 0);

            // Verify the RHS (credit) entry
            assert_eq!(published_payload.rhs[0].entry_type, AccountingEntryType::Credit);
            assert_eq!(published_payload.rhs[0].account, format!("AccountCreationFee#{}", payload.state_hash));
            assert_eq!(published_payload.rhs[0].account_type, AccountingEntryAccountType::VirtualAddess);
            assert_eq!(published_payload.rhs[0].amount_nanomina, 1_000_000_000);
            assert_eq!(published_payload.rhs[0].timestamp, 0);
        } else {
            panic!("Expected a DoubleEntryTransaction event to be published.");
        }

        assert_eq!(actor.entries_processed.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_process_new_account_event_at_height_0() {
        let (actor, mut receiver) = setup_actor();

        // Mock NewAccountPayload
        let payload = NewAccountPayload {
            height: 0,
            state_hash: "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ".to_string(),
            timestamp: 0,
            account: "B62qnewaccount1".to_string(),
            apply: true,
        };

        // Create the event
        let event = Event {
            event_type: EventType::NewAccount,
            payload: sonic_rs::to_string(&payload).unwrap(),
        };

        // Handle the NewAccount event
        actor.handle_event(event).await;

        // Verify no published DoubleEntryTransaction event
        let published_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(published_event.is_err(), "Did not expected a DoubleEntryTransaction event to be published.");

        assert_eq!(actor.entries_processed.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_process_user_command_stake_delegation() {
        let (actor, mut receiver) = setup_actor();

        let payload = CanonicalUserCommandLogPayload {
            height: 200,
            global_slot: 0,
            memo: "halp".to_string(),
            txn_hash: "txn_hash".to_string(),
            state_hash: "state_hash_3".to_string(),
            timestamp: 1620000200,
            txn_type: CommandType::StakeDelegation,
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

        actor.process_user_commands(&payload).await;

        // Verify the fee transaction (fee payer to block reward pool)
        let published_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(
            published_event.is_ok(),
            "Expected the DoubleEntryTransaction event for the fee to be published."
        );

        if let Ok(Ok(event)) = published_event {
            let published_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.payload).unwrap();
            assert_eq!(published_payload.height, payload.height);

            // Verify the fee transaction
            assert_eq!(published_payload.lhs.len(), 1);
            assert_eq!(published_payload.rhs.len(), 1);

            // Debit: Fee payer
            assert_eq!(published_payload.lhs[0].entry_type, AccountingEntryType::Debit);
            assert_eq!(published_payload.lhs[0].account, payload.fee_payer);
            assert_eq!(published_payload.lhs[0].amount_nanomina, payload.fee_nanomina);

            // Credit: Block reward pool
            assert_eq!(published_payload.rhs[0].entry_type, AccountingEntryType::Credit);
            assert_eq!(published_payload.rhs[0].account, format!("BlockRewardPool#{}", payload.state_hash));
            assert_eq!(published_payload.rhs[0].amount_nanomina, payload.fee_nanomina);
        }

        // Verify that no balance transfer transaction is published for stake delegations
        let balance_transfer_event = timeout(std::time::Duration::from_secs(1), receiver.recv()).await;
        assert!(
            balance_transfer_event.is_err(),
            "No balance transfer transaction should be published for a stake delegation."
        );

        assert_eq!(actor.entries_processed.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_process_canonical_batch_zkapp_command_log() {
        let (actor, mut receiver) = setup_actor();

        let payload = CanonicalBatchZkappCommandLogPayload {
            canonical: true,
            was_canonical: false,
            height: 400,
            state_hash: "state_hash_7".to_string(),
            timestamp: 1620000600,
            global_slot: 1000,
            commands: vec![
                ZkAppCommandSummary {
                    memo: "memo_1".to_string(),
                    fee_payer: "B62qfee_payer1".to_string(),
                    status: CommandStatus::Applied,
                    txn_type: CommandType::Payment,
                    nonce: 1,
                    fee_nanomina: 10_000,
                    account_updates: 2,
                },
                ZkAppCommandSummary {
                    memo: "memo_2".to_string(),
                    fee_payer: "B62qfee_payer2".to_string(),
                    status: CommandStatus::Failed,
                    txn_type: CommandType::StakeDelegation,
                    nonce: 2,
                    fee_nanomina: 20_000,
                    account_updates: 3,
                },
            ],
        };

        actor
            .handle_event(Event {
                event_type: EventType::CanonicalBatchZkappCommandLog,
                payload: sonic_rs::to_string(&payload).unwrap(),
            })
            .await;

        // Verify the published transactions for each command in the batch
        for command in &payload.commands {
            let published_event = tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv())
                .await
                .expect("Expected a DoubleEntryTransaction event")
                .expect("Event received");

            let published_payload: DoubleEntryRecordPayload = sonic_rs::from_str(&published_event.payload).unwrap();

            // Ensure the published transaction matches the height and state hash
            assert_eq!(published_payload.height, payload.height);
            assert_eq!(published_payload.state_hash, payload.state_hash);

            // Verify the fee transaction for the fee payer
            assert_eq!(published_payload.lhs.len(), 1);
            assert_eq!(published_payload.rhs.len(), 1);

            assert_eq!(published_payload.lhs[0].entry_type, AccountingEntryType::Debit);
            assert_eq!(published_payload.lhs[0].account, command.fee_payer);
            assert_eq!(published_payload.lhs[0].amount_nanomina, command.fee_nanomina);

            assert_eq!(published_payload.rhs[0].entry_type, AccountingEntryType::Credit);
            assert_eq!(published_payload.rhs[0].account, format!("BlockRewardPool#{}", payload.state_hash));
            assert_eq!(published_payload.rhs[0].amount_nanomina, command.fee_nanomina);
        }

        assert_eq!(
            actor.entries_processed.load(std::sync::atomic::Ordering::SeqCst),
            payload.commands.len(),
            "Expected the number of processed entries to match the number of commands in the batch"
        );
    }

    #[tokio::test]
    async fn test_process_non_canonical_batch_zkapp_command_log() {
        let (actor, mut receiver) = setup_actor();

        let payload = CanonicalBatchZkappCommandLogPayload {
            canonical: false,
            was_canonical: false,
            height: 500,
            state_hash: "state_hash_8".to_string(),
            timestamp: 1620000700,
            global_slot: 2000,
            commands: vec![ZkAppCommandSummary {
                memo: "memo_3".to_string(),
                fee_payer: "B62qfee_payer3".to_string(),
                status: CommandStatus::Applied,
                txn_type: CommandType::Payment,
                nonce: 1,
                fee_nanomina: 15_000,
                account_updates: 2,
            }],
        };

        actor
            .handle_event(Event {
                event_type: EventType::CanonicalBatchZkappCommandLog,
                payload: sonic_rs::to_string(&payload).unwrap(),
            })
            .await;

        // Verify no event is published for non-canonical payloads
        let published_event = tokio::time::timeout(std::time::Duration::from_millis(100), receiver.recv()).await;
        assert!(
            published_event.is_err(),
            "No event should be published for non-canonical batch zkapp command logs"
        );

        assert_eq!(
            actor.entries_processed.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "No entries should be processed for non-canonical batch zkapp command logs"
        );
    }
}
