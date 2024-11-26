use super::super::{
    events::{Event, EventType},
    shared_publisher::SharedPublisher,
    Actor,
};
use crate::{
    constants::MAINNET_EPOCH_SLOT_COUNT,
    stream::{mainnet_block_models::CommandType, payloads::*},
};
use async_trait::async_trait;
use std::sync::{atomic::AtomicUsize, Arc};

pub struct StakingAccountingActor {
    pub id: String,
    pub shared_publisher: Arc<SharedPublisher>,
    pub events_published: AtomicUsize,
}

#[allow(dead_code)]
impl StakingAccountingActor {
    pub fn new(shared_publisher: Arc<SharedPublisher>) -> Self {
        Self {
            id: "StakingAccountingActor".to_string(),
            shared_publisher,
            events_published: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Actor for StakingAccountingActor {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn actor_outputs(&self) -> &AtomicUsize {
        &self.events_published
    }

    async fn handle_event(&self, event: Event) {
        if event.event_type == EventType::CanonicalUserCommandLog {
            let log: CanonicalUserCommandLogPayload = sonic_rs::from_str(&event.payload).unwrap();
            if log.txn_type == CommandType::StakeDelegation {
                // not canonical, and wasn't before. No need to publish
                if !log.canonical && !log.was_canonical {
                    return;
                }
                let epoch = log.global_slot / MAINNET_EPOCH_SLOT_COUNT;
                let mut payload = DoubleEntryRecordPayload {
                    height: log.height,
                    state_hash: log.state_hash,
                    ledger_destination: LedgerDestination::StakingLedger,
                    lhs: vec![AccountingEntry {
                        transfer_type: CommandType::StakeDelegation.to_string(),
                        account: log.sender.to_string(),
                        counterparty: log.receiver.to_string(),
                        entry_type: AccountingEntryType::Debit,
                        account_type: AccountingEntryAccountType::BlockchainAddress,
                        amount_nanomina: log.amount_nanomina,
                        timestamp: epoch,
                    }],
                    rhs: vec![AccountingEntry {
                        transfer_type: CommandType::StakeDelegation.to_string(),
                        account: log.receiver,
                        counterparty: log.sender,
                        entry_type: AccountingEntryType::Credit,
                        account_type: AccountingEntryAccountType::BlockchainAddress,
                        amount_nanomina: log.amount_nanomina,
                        timestamp: epoch,
                    }],
                };
                if !log.canonical {
                    // swap debit and credit if non-canonical
                    payload.lhs[0].entry_type = AccountingEntryType::Credit;
                    payload.rhs[0].entry_type = AccountingEntryType::Debit;
                }
                self.publish(Event {
                    event_type: EventType::DoubleEntryTransaction,
                    payload: sonic_rs::to_string(&payload).unwrap(),
                })
            }
        }
    }

    fn publish(&self, event: Event) {
        self.incr_event_published();
        self.shared_publisher.publish(event);
    }
}

#[cfg(test)]
mod staking_accounting_actor_tests {
    use super::*;
    use crate::stream::{
        events::{Event, EventType},
        payloads::{AccountingEntryType, CanonicalUserCommandLogPayload, DoubleEntryRecordPayload},
    };
    use std::sync::Arc;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_handle_event_stake_delegation() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = StakingAccountingActor::new(Arc::clone(&shared_publisher));
        let mut receiver = shared_publisher.subscribe();

        // Create a stake delegation log event
        let canonical_event = Event {
            event_type: EventType::CanonicalUserCommandLog,
            payload: sonic_rs::to_string(&CanonicalUserCommandLogPayload {
                txn_type: CommandType::StakeDelegation,
                global_slot: 14280,
                height: 50,
                state_hash: "state_hash_123".to_string(),
                sender: "source_address".to_string(),
                receiver: "recipient_address".to_string(),
                amount_nanomina: 1000000,
                canonical: true,
                was_canonical: false,
                ..Default::default()
            })
            .unwrap(),
        };

        actor.handle_event(canonical_event).await;

        // Verify that the DoubleEntryTransaction event is published
        if let Ok(event) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            let payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.unwrap().payload).unwrap();
            assert_eq!(payload.height, 50);
            assert_eq!(payload.state_hash, "state_hash_123");
            assert_eq!(payload.lhs[0].account, "source_address");
            assert_eq!(payload.lhs[0].counterparty, "recipient_address");
            assert_eq!(payload.lhs[0].amount_nanomina, 1000000);
            assert_eq!(payload.lhs[0].entry_type, AccountingEntryType::Debit);
            assert_eq!(payload.rhs[0].account, "recipient_address");
            assert_eq!(payload.rhs[0].counterparty, "source_address");
            assert_eq!(payload.rhs[0].amount_nanomina, 1000000);
            assert_eq!(payload.rhs[0].entry_type, AccountingEntryType::Credit);
        } else {
            panic!("Expected DoubleEntryTransaction event not published");
        }
    }

    #[tokio::test]
    async fn test_handle_event_non_canonical_was_canonical_stake_delegation() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = StakingAccountingActor::new(Arc::clone(&shared_publisher));
        let mut receiver = shared_publisher.subscribe();

        // Create a non-canonical stake delegation log event with `was_canonical = true`
        let non_canonical_event = Event {
            event_type: EventType::CanonicalUserCommandLog,
            payload: sonic_rs::to_string(&CanonicalUserCommandLogPayload {
                txn_type: CommandType::StakeDelegation,
                global_slot: 14280,
                height: 50,
                state_hash: "state_hash_123".to_string(),
                sender: "source_address".to_string(),
                receiver: "recipient_address".to_string(),
                amount_nanomina: 1000000,
                canonical: false,
                was_canonical: true,
                ..Default::default()
            })
            .unwrap(),
        };

        actor.handle_event(non_canonical_event).await;

        // Verify that the DoubleEntryTransaction event is published
        if let Ok(event) = timeout(std::time::Duration::from_secs(1), receiver.recv()).await {
            let payload: DoubleEntryRecordPayload = sonic_rs::from_str(&event.unwrap().payload).unwrap();
            assert_eq!(payload.height, 50);
            assert_eq!(payload.state_hash, "state_hash_123");
            assert_eq!(payload.lhs[0].account, "source_address");
            assert_eq!(payload.lhs[0].counterparty, "recipient_address");
            assert_eq!(payload.lhs[0].amount_nanomina, 1000000);
            assert_eq!(
                payload.lhs[0].entry_type,
                AccountingEntryType::Credit,
                "Non-canonical should swap lhs to Credit"
            );
            assert_eq!(payload.rhs[0].account, "recipient_address");
            assert_eq!(payload.rhs[0].counterparty, "source_address");
            assert_eq!(payload.rhs[0].amount_nanomina, 1000000);
            assert_eq!(payload.rhs[0].entry_type, AccountingEntryType::Debit, "Non-canonical should swap rhs to Debit");
        } else {
            panic!("Expected DoubleEntryTransaction event not published");
        }
    }

    #[tokio::test]
    async fn test_handle_event_non_canonical_stake_delegation() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = StakingAccountingActor::new(Arc::clone(&shared_publisher));
        let mut receiver = shared_publisher.subscribe();

        // Create a non-canonical stake delegation log event
        let non_canonical_event = Event {
            event_type: EventType::CanonicalUserCommandLog,
            payload: sonic_rs::to_string(&CanonicalUserCommandLogPayload {
                txn_type: CommandType::StakeDelegation,
                global_slot: 14280,
                height: 50,
                state_hash: "state_hash_123".to_string(),
                sender: "source_address".to_string(),
                receiver: "recipient_address".to_string(),
                amount_nanomina: 1000000,
                canonical: false,
                was_canonical: false,
                ..Default::default()
            })
            .unwrap(),
        };

        actor.handle_event(non_canonical_event).await;

        // Verify that no DoubleEntryTransaction event is published
        assert!(
            timeout(std::time::Duration::from_millis(100), receiver.recv()).await.is_err(),
            "No event should be published for non-canonical stake delegation"
        );
    }

    #[tokio::test]
    async fn test_handle_event_non_stake_delegation() {
        let shared_publisher = Arc::new(SharedPublisher::new(100));
        let actor = StakingAccountingActor::new(Arc::clone(&shared_publisher));
        let mut receiver = shared_publisher.subscribe();

        // Create a non-stake delegation log event
        let non_stake_delegation_event = Event {
            event_type: EventType::CanonicalUserCommandLog,
            payload: sonic_rs::to_string(&CanonicalUserCommandLogPayload {
                txn_type: CommandType::Payment,
                global_slot: 14280,
                height: 50,
                state_hash: "state_hash_123".to_string(),
                sender: "source_address".to_string(),
                receiver: "recipient_address".to_string(),
                amount_nanomina: 1000000,
                canonical: true,
                was_canonical: false,
                ..Default::default()
            })
            .unwrap(),
        };

        actor.handle_event(non_stake_delegation_event).await;

        // Verify that no DoubleEntryTransaction event is published
        assert!(
            timeout(std::time::Duration::from_millis(100), receiver.recv()).await.is_err(),
            "No event should be published for non-stake delegation transactions"
        );
    }
}
