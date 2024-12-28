use crate::event_sourcing::{
    actor_dag::{ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
    events::{Event, EventType},
    models::{CommandSummary, CommandType, FeeTransfer, FeeTransferViaCoinbase, ZkAppCommandSummary},
    payloads::{
        AccountingEntry, AccountingEntryAccountType, AccountingEntryType, BerkeleyBlockPayload, CanonicalBerkeleyBlockPayload, CanonicalMainnetBlockPayload,
        DoubleEntryRecordPayload, InternalCommandType, LedgerDestination, MainnetBlockPayload, NewAccountPayload,
    },
};
use async_trait::async_trait;

pub struct AccountingActor;

/// A trait that unifies the data needed to do accounting:
///   - state_hash
///   - timestamp
///   - coinbase receiver + reward
///   - user commands
///   - fee transfers
///   - fee_transfer_via_coinbase
///   - plus (for Berkeley) optional zk_app_commands
pub trait AccountingBlock {
    fn get_state_hash(&self) -> &str;
    fn get_timestamp(&self) -> u64;
    fn get_coinbase_receiver(&self) -> &str;
    fn get_coinbase_reward(&self) -> u64;
    fn get_user_commands(&self) -> &[CommandSummary];
    fn get_fee_transfers(&self) -> &[FeeTransfer];
    fn get_fee_transfer_via_coinbase(&self) -> Option<&[FeeTransferViaCoinbase]>;

    // For Berkeley blocks, if you have zk_app_commands:
    fn get_zk_app_commands(&self) -> Option<&[ZkAppCommandSummary]> {
        None // default
    }
}

// --------------------------------------
// 2) Implement the trait for MainnetBlockPayload
// --------------------------------------

impl AccountingBlock for MainnetBlockPayload {
    fn get_state_hash(&self) -> &str {
        &self.state_hash
    }
    fn get_timestamp(&self) -> u64 {
        self.timestamp
    }
    fn get_coinbase_receiver(&self) -> &str {
        &self.coinbase_receiver
    }
    fn get_coinbase_reward(&self) -> u64 {
        self.coinbase_reward_nanomina
    }
    fn get_user_commands(&self) -> &[CommandSummary] {
        &self.user_commands
    }
    fn get_fee_transfers(&self) -> &[FeeTransfer] {
        &self.fee_transfers
    }
    fn get_fee_transfer_via_coinbase(&self) -> Option<&[FeeTransferViaCoinbase]> {
        self.fee_transfer_via_coinbase.as_deref()
    }
}

// --------------------------------------
// 3) Implement the trait for BerkeleyBlockPayload
// --------------------------------------

impl AccountingBlock for BerkeleyBlockPayload {
    fn get_state_hash(&self) -> &str {
        &self.state_hash
    }
    fn get_timestamp(&self) -> u64 {
        self.timestamp
    }
    fn get_coinbase_receiver(&self) -> &str {
        &self.coinbase_receiver
    }
    fn get_coinbase_reward(&self) -> u64 {
        self.coinbase_reward_nanomina
    }
    fn get_user_commands(&self) -> &[CommandSummary] {
        &self.user_commands
    }
    fn get_fee_transfers(&self) -> &[FeeTransfer] {
        &self.fee_transfers
    }
    fn get_fee_transfer_via_coinbase(&self) -> Option<&[FeeTransferViaCoinbase]> {
        self.fee_transfer_via_coinbase.as_deref()
    }
    fn get_zk_app_commands(&self) -> Option<&[ZkAppCommandSummary]> {
        Some(&self.zk_app_commands)
    }
}

impl AccountingActor {
    /// Your partial method #1
    async fn process_fee_transfer(
        state_hash: &str,
        timestamp: u64,
        fee_transfer: FeeTransfer,
        canonical: bool,
    ) -> (Vec<AccountingEntry>, Vec<AccountingEntry>) {
        // (Identical to your partial refactor code)
        let mut source = AccountingEntry {
            transfer_type: InternalCommandType::FeeTransfer.to_string(),
            counterparty: fee_transfer.recipient.to_string(),
            entry_type: AccountingEntryType::Debit,
            account: format!("BlockRewardPool#{}", state_hash),
            account_type: crate::event_sourcing::payloads::AccountingEntryAccountType::VirtualAddess,
            amount_nanomina: fee_transfer.fee_nanomina,
            timestamp,
        };
        let mut recipient = AccountingEntry {
            transfer_type: InternalCommandType::FeeTransfer.to_string(),
            counterparty: format!("BlockRewardPool#{}", state_hash),
            entry_type: AccountingEntryType::Credit,
            account: fee_transfer.recipient.clone(),
            account_type: crate::event_sourcing::payloads::AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: fee_transfer.fee_nanomina,
            timestamp,
        };

        // If canonical == false => swap
        if !canonical {
            source.entry_type = AccountingEntryType::Credit;
            recipient.entry_type = AccountingEntryType::Debit;
        }

        (vec![source], vec![recipient])
    }

    async fn process_coinbase(
        state_hash: &str,
        timestamp: u64,
        coinbase_receiver: &str,
        coinbase_reward: u64,
        canonical: bool,
    ) -> (Vec<AccountingEntry>, Vec<AccountingEntry>) {
        let mut source = AccountingEntry {
            transfer_type: InternalCommandType::Coinbase.to_string(),
            counterparty: coinbase_receiver.to_string(),
            entry_type: AccountingEntryType::Debit,
            account: format!("MinaCoinbasePayment#{}", state_hash),
            account_type: AccountingEntryAccountType::VirtualAddess,
            amount_nanomina: coinbase_reward,
            timestamp,
        };
        let mut recipient = AccountingEntry {
            transfer_type: InternalCommandType::Coinbase.to_string(),
            counterparty: format!("MinaCoinbasePayment#{}", state_hash),
            entry_type: AccountingEntryType::Credit,
            account: coinbase_receiver.to_string(),
            account_type: AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: coinbase_reward,
            timestamp,
        };

        // If canonical == false => swap
        if !canonical {
            source.entry_type = AccountingEntryType::Credit;
            recipient.entry_type = AccountingEntryType::Debit;
        }

        (vec![source], vec![recipient])
    }

    /// Your partial method #2
    async fn process_fee_transfer_via_coinbase(
        state_hash: &str,
        timestamp: u64,
        coinbase_receiver: &str,
        fee_transfer_via_coinbase: &FeeTransferViaCoinbase,
        canonical: bool,
    ) -> (Vec<AccountingEntry>, Vec<AccountingEntry>) {
        // (Identical to your partial refactor code)
        let mut lhs = vec![];
        let mut rhs = vec![];

        // 1) "BlockRewardPool" side
        let mut source = AccountingEntry {
            transfer_type: "BlockRewardPool".to_string(),
            counterparty: format!("BlockRewardPool#{}", state_hash),
            entry_type: AccountingEntryType::Debit,
            account: coinbase_receiver.to_string(),
            account_type: crate::event_sourcing::payloads::AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: fee_transfer_via_coinbase.fee_nanomina,
            timestamp,
        };
        let mut recipient = AccountingEntry {
            transfer_type: "BlockRewardPool".to_string(),
            counterparty: coinbase_receiver.to_string(),
            entry_type: AccountingEntryType::Credit,
            account: format!("BlockRewardPool#{}", state_hash),
            account_type: crate::event_sourcing::payloads::AccountingEntryAccountType::VirtualAddess,
            amount_nanomina: fee_transfer_via_coinbase.fee_nanomina,
            timestamp,
        };

        if !canonical {
            source.entry_type = AccountingEntryType::Credit;
            recipient.entry_type = AccountingEntryType::Debit;
        }
        lhs.push(source);
        rhs.push(recipient);

        // 2) "FeeTransferViaCoinbase" side
        let mut source = AccountingEntry {
            transfer_type: InternalCommandType::FeeTransferViaCoinbase.to_string(),
            counterparty: fee_transfer_via_coinbase.receiver.to_string(),
            entry_type: AccountingEntryType::Debit,
            account: format!("BlockRewardPool#{}", state_hash),
            account_type: crate::event_sourcing::payloads::AccountingEntryAccountType::VirtualAddess,
            amount_nanomina: fee_transfer_via_coinbase.fee_nanomina,
            timestamp,
        };
        let mut recipient = AccountingEntry {
            transfer_type: InternalCommandType::FeeTransferViaCoinbase.to_string(),
            counterparty: format!("BlockRewardPool#{}", state_hash),
            entry_type: AccountingEntryType::Credit,
            account: fee_transfer_via_coinbase.receiver.to_string(),
            account_type: crate::event_sourcing::payloads::AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: fee_transfer_via_coinbase.fee_nanomina,
            timestamp,
        };
        if !canonical {
            source.entry_type = AccountingEntryType::Credit;
            recipient.entry_type = AccountingEntryType::Debit;
        }
        lhs.push(source);
        rhs.push(recipient);

        (lhs, rhs)
    }

    /// Your partial method #3
    async fn process_user_command(state_hash: &str, timestamp: u64, command: &CommandSummary, canonical: bool) -> (Vec<AccountingEntry>, Vec<AccountingEntry>) {
        // (Identical to your partial refactor code)
        let mut lhs = vec![];
        let mut rhs = vec![];

        // sender => receiver
        let mut sender_entry = AccountingEntry {
            transfer_type: command.txn_type.to_string(),
            counterparty: command.receiver.to_string(),
            entry_type: AccountingEntryType::Debit,
            account: command.sender.to_string(),
            account_type: crate::event_sourcing::payloads::AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: command.amount_nanomina,
            timestamp,
        };
        let mut receiver_entry = AccountingEntry {
            transfer_type: command.txn_type.to_string(),
            counterparty: command.sender.to_string(),
            entry_type: AccountingEntryType::Credit,
            account: command.receiver.to_string(),
            account_type: crate::event_sourcing::payloads::AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: command.amount_nanomina,
            timestamp,
        };
        if !canonical {
            sender_entry.entry_type = AccountingEntryType::Credit;
            receiver_entry.entry_type = AccountingEntryType::Debit;
        }
        // If the command is Applied and not StakeDelegation => push
        if command.status == crate::event_sourcing::models::CommandStatus::Applied && command.txn_type != CommandType::StakeDelegation {
            lhs.push(sender_entry);
            rhs.push(receiver_entry);
        }

        // fee payer => block reward pool
        let mut fee_payer_entry = AccountingEntry {
            counterparty: format!("BlockRewardPool#{}", state_hash),
            transfer_type: "BlockRewardPool".to_string(),
            entry_type: AccountingEntryType::Debit,
            account: command.fee_payer.to_string(),
            account_type: crate::event_sourcing::payloads::AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: command.fee_nanomina,
            timestamp,
        };
        let mut block_reward_pool_entry = AccountingEntry {
            counterparty: command.fee_payer.to_string(),
            transfer_type: "BlockRewardPool".to_string(),
            entry_type: AccountingEntryType::Credit,
            account: format!("BlockRewardPool#{}", state_hash),
            account_type: crate::event_sourcing::payloads::AccountingEntryAccountType::VirtualAddess,
            amount_nanomina: command.fee_nanomina,
            timestamp,
        };

        if !canonical {
            fee_payer_entry.entry_type = AccountingEntryType::Credit;
            block_reward_pool_entry.entry_type = AccountingEntryType::Debit;
        }
        lhs.push(fee_payer_entry);
        rhs.push(block_reward_pool_entry);

        (lhs, rhs)
    }

    /// Your partial method #4
    async fn process_batch_zk_app_commands(
        state_hash: &str,
        timestamp: u64,
        command: &ZkAppCommandSummary,
        canonical: bool,
    ) -> (Vec<AccountingEntry>, Vec<AccountingEntry>) {
        // (Identical to your partial refactor code)
        let mut lhs = vec![];
        let mut rhs = vec![];

        // fee payer => block reward pool
        let mut fee_payer_entry = AccountingEntry {
            counterparty: format!("BlockRewardPool#{}", state_hash),
            transfer_type: "BlockRewardPool".to_string(),
            entry_type: AccountingEntryType::Debit,
            account: command.fee_payer.to_string(),
            account_type: crate::event_sourcing::payloads::AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: command.fee_nanomina,
            timestamp,
        };
        let mut block_reward_pool_entry = AccountingEntry {
            counterparty: command.fee_payer.to_string(),
            transfer_type: "BlockRewardPool".to_string(),
            entry_type: AccountingEntryType::Credit,
            account: format!("BlockRewardPool#{}", state_hash),
            account_type: crate::event_sourcing::payloads::AccountingEntryAccountType::VirtualAddess,
            amount_nanomina: command.fee_nanomina,
            timestamp,
        };

        if !canonical {
            fee_payer_entry.entry_type = AccountingEntryType::Credit;
            block_reward_pool_entry.entry_type = AccountingEntryType::Debit;
        }
        lhs.push(fee_payer_entry);
        rhs.push(block_reward_pool_entry);

        (lhs, rhs)
    }

    async fn process_generic_block<B: AccountingBlock>(block: &B, canonical: bool) -> (Vec<AccountingEntry>, Vec<AccountingEntry>) {
        let mut total_lhs = Vec::new();
        let mut total_rhs = Vec::new();

        // 4a) user commands
        for cmd in block.get_user_commands() {
            let (lhs, rhs) = Self::process_user_command(block.get_state_hash(), block.get_timestamp(), cmd, canonical).await;
            total_lhs.extend(lhs);
            total_rhs.extend(rhs);
        }

        // 4b) possible zk_app_commands (only meaningful for Berkeley)
        if let Some(zk_cmds) = block.get_zk_app_commands() {
            for cmd in zk_cmds {
                let (lhs, rhs) = Self::process_batch_zk_app_commands(block.get_state_hash(), block.get_timestamp(), cmd, canonical).await;
                total_lhs.extend(lhs);
                total_rhs.extend(rhs);
            }
        }

        // 4c) fee transfers
        for ft in block.get_fee_transfers() {
            let (lhs, rhs) = Self::process_fee_transfer(
                block.get_state_hash(),
                block.get_timestamp(),
                ft.clone(), // FeeTransfer is Copy or clone it
                canonical,
            )
            .await;
            total_lhs.extend(lhs);
            total_rhs.extend(rhs);
        }

        // 4d) fee_transfer_via_coinbase
        if let Some(ftvc) = block.get_fee_transfer_via_coinbase() {
            for xfer in ftvc {
                let (lhs, rhs) =
                    Self::process_fee_transfer_via_coinbase(block.get_state_hash(), block.get_timestamp(), block.get_coinbase_receiver(), xfer, canonical)
                        .await;
                total_lhs.extend(lhs);
                total_rhs.extend(rhs);
            }
        }

        // 4e) coinbase
        let (lhs_coinbase, rhs_coinbase) = Self::process_coinbase(
            block.get_state_hash(),
            block.get_timestamp(),
            block.get_coinbase_receiver(),
            block.get_coinbase_reward(),
            canonical,
        )
        .await;
        total_lhs.extend(lhs_coinbase);
        total_rhs.extend(rhs_coinbase);

        (total_lhs, total_rhs)
    }
}

#[async_trait]
impl ActorFactory for AccountingActor {
    async fn create_actor() -> ActorNode {
        ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|event, _state, _requeue| {
                Box::pin(async move {
                    match event.event_type {
                        EventType::NewAccount => {
                            let payload: NewAccountPayload = sonic_rs::from_str(&event.payload).unwrap();
                            if payload.height < 2 {
                                // genesis ledger accounts pay no account creation fees
                                // magic mina receiver in block 1 is also no subject to account creation fee
                                return None;
                            }
                            let record = DoubleEntryRecordPayload {
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

                            let new_event = Event {
                                event_type: EventType::DoubleEntryTransaction,
                                payload: sonic_rs::to_string(&record).unwrap(),
                            };

                            Some(vec![new_event])
                        }
                        EventType::CanonicalMainnetBlock => {
                            // parse payload
                            let payload: CanonicalMainnetBlockPayload = sonic_rs::from_str(&event.payload).expect("Failed to parse MainnetBlockPayload");

                            // check (canonical, was_canonical)
                            if !payload.canonical && !payload.was_canonical {
                                return None;
                            }

                            // unify logic
                            let (lhs, rhs) = Self::process_generic_block(&payload.block, payload.canonical).await;

                            if lhs.is_empty() && rhs.is_empty() {
                                return None;
                            }

                            let record = DoubleEntryRecordPayload {
                                height: payload.block.height,
                                state_hash: payload.block.state_hash.clone(),
                                ledger_destination: LedgerDestination::BlockchainLedger,
                                lhs,
                                rhs,
                            };
                            record.verify();

                            let new_event = Event {
                                event_type: EventType::DoubleEntryTransaction,
                                payload: sonic_rs::to_string(&record).unwrap(),
                            };

                            Some(vec![new_event])
                        }

                        EventType::CanonicalBerkeleyBlock => {
                            let payload: CanonicalBerkeleyBlockPayload = sonic_rs::from_str(&event.payload).expect("Failed to parse BerkeleyBlockPayload");

                            if !payload.canonical && !payload.was_canonical {
                                return None;
                            }

                            // also calls the same function
                            let (lhs, rhs) = Self::process_generic_block(&payload.block, payload.canonical).await;

                            if lhs.is_empty() && rhs.is_empty() {
                                return None;
                            }

                            let record = DoubleEntryRecordPayload {
                                height: payload.block.height,
                                state_hash: payload.block.state_hash.clone(),
                                ledger_destination: LedgerDestination::BlockchainLedger,
                                lhs,
                                rhs,
                            };
                            record.verify();

                            let new_event = Event {
                                event_type: EventType::DoubleEntryTransaction,
                                payload: sonic_rs::to_string(&record).unwrap(),
                            };
                            Some(vec![new_event])
                        }

                        _ => None,
                    }
                })
            })
            .build()
    }
}

#[cfg(test)]
mod accounting_actor_tests_v2 {
    use super::AccountingActor;
    use crate::{
        constants::MAINNET_COINBASE_REWARD,
        event_sourcing::{
            actor_dag::{ActorDAG, ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
            events::{Event, EventType},
            models::{CommandSummary, FeeTransfer, FeeTransferViaCoinbase},
            payloads::{AccountingEntryType, CanonicalMainnetBlockPayload, DoubleEntryRecordPayload, LedgerDestination, MainnetBlockPayload},
        },
    };
    use std::sync::Arc;
    use tokio::{
        sync::{watch, Mutex},
        time::{sleep, Duration},
    };

    // ------------------------------
    // SINK NODE + HELPER FUNCTIONS
    // ------------------------------

    /// This node captures `DoubleEntryTransaction` events, storing them in a vector
    /// under the key `"captured_transactions"` in its ActorStore.
    fn create_double_entry_sink_node() -> ActorNode {
        ActorNodeBuilder::new()
            .with_state(ActorStore::new())
            .with_processor(|event, state, _requeue| {
                Box::pin(async move {
                    if event.event_type == EventType::DoubleEntryTransaction {
                        let mut store = state.lock().await;
                        let mut captured: Vec<String> = store.get("captured_transactions").cloned().unwrap_or_default();
                        captured.push(event.payload.clone());
                        store.insert("captured_transactions", captured);
                    }
                    None
                })
            })
            .build()
    }

    /// Helper to read the sink node’s captured DoubleEntryTransaction events (as JSON).
    async fn read_captured_transactions(dag: &Arc<Mutex<ActorDAG>>, sink_node_id: &str) -> Vec<String> {
        let dag_locked = dag.lock().await;
        let sink_node_locked = dag_locked.read_node(sink_node_id.to_string()).expect("Sink node not found").lock().await;
        let store = sink_node_locked.get_state();
        let store_locked = store.lock().await;
        store_locked.get::<Vec<String>>("captured_transactions").cloned().unwrap_or_default()
    }

    // ------------------------------
    // TEST
    // ------------------------------

    #[tokio::test]
    async fn test_accounting_actor_single_fee_transfer_with_sink() {
        // 2) Build the ActorDAG
        let mut dag = ActorDAG::new();

        // 3) Create the AccountingActor (root)
        let accounting_actor = AccountingActor::create_actor().await;
        let actor_id = accounting_actor.id();
        // set_root returns a Sender<Event>
        let actor_sender = dag.set_root(accounting_actor);

        // 4) Create the sink node, add to DAG, link from AccountingActor
        let sink_node = create_double_entry_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, sink_node_id);

        // 5) Wrap + spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 6) Construct a single-fee-transfer CanonicalMainnetBlock
        let test_fee_transfer = FeeTransfer {
            recipient: "B62qtestrecipient".to_string(),
            fee_nanomina: 42_000_000_000, // e.g., 42 mina
        };

        let test_block = MainnetBlockPayload {
            height: 999,
            state_hash: "state_hash_test_fee".to_string(),
            previous_state_hash: "prev_hash".to_string(),
            last_vrf_output: "some_vrf_output".to_string(),
            user_command_count: 0,
            internal_command_count: 0,
            user_commands: vec![], // no user commands
            snark_work_count: 0,
            snark_work: vec![],
            timestamp: 123456789,
            coinbase_receiver: "B62qpcoinbaseReceiver".to_string(),
            coinbase_reward_nanomina: MAINNET_COINBASE_REWARD,
            global_slot_since_genesis: 777,
            fee_transfer_via_coinbase: None,
            fee_transfers: vec![test_fee_transfer],
            global_slot: 777,
        };

        let canonical_payload = CanonicalMainnetBlockPayload {
            block: test_block,
            canonical: true,      // so it will be processed
            was_canonical: false, // not previously canonical
        };

        // 7) Send the event to the actor
        actor_sender
            .send(Event {
                event_type: EventType::CanonicalMainnetBlock,
                payload: sonic_rs::to_string(&canonical_payload).unwrap(),
            })
            .await
            .expect("Failed to send event");

        // Wait for processing
        sleep(Duration::from_millis(200)).await;

        // 8) Check the sink node
        let transactions = read_captured_transactions(&dag, sink_node_id).await;
        assert_eq!(transactions.len(), 1, "Expected exactly one DoubleEntryTransaction event in the sink.");

        // 9) Parse and verify
        let record_json = &transactions[0];
        let record: DoubleEntryRecordPayload = sonic_rs::from_str(record_json).expect("Failed to parse DoubleEntryRecordPayload");
        assert_eq!(record.height, 999);
        assert_eq!(record.state_hash, "state_hash_test_fee");
        assert_eq!(record.ledger_destination, LedgerDestination::BlockchainLedger);

        // Expect a single LHS + single RHS
        assert_eq!(record.lhs.len(), 2, "Should be 2 debit entry (including coinbase)");
        assert_eq!(record.rhs.len(), 2, "Should be 2 credit entry (including coinbase)");

        // Verify the single LHS entry
        let lhs = &record.lhs[0];
        assert_eq!(lhs.entry_type, AccountingEntryType::Debit);
        assert_eq!(lhs.account, "BlockRewardPool#state_hash_test_fee");
        assert_eq!(lhs.amount_nanomina, 42_000_000_000);

        // Verify the single RHS entry
        let rhs = &record.rhs[0];
        assert_eq!(rhs.entry_type, AccountingEntryType::Credit);
        assert_eq!(rhs.account, "B62qtestrecipient");
        assert_eq!(rhs.amount_nanomina, 42_000_000_000);
    }

    #[tokio::test]
    async fn test_non_canonical_fee_transfer_with_coinbase() {
        // 1) Create the shutdown signal
        let (_shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2) Build the ActorDAG
        let mut dag = ActorDAG::new();

        // 3) Create the AccountingActor (root)
        let accounting_actor = AccountingActor::create_actor().await;
        let actor_id = accounting_actor.id();
        // set_root returns a Sender<Event>
        let actor_sender = dag.set_root(accounting_actor);

        // 4) Create the sink node, add to DAG, link from AccountingActor
        // Assume you already have a create_double_entry_sink_node(...) function
        let sink_node = create_double_entry_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, sink_node_id);

        // 5) Wrap + spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 6) Construct a MainnetBlockPayload with:
        //    - coinbase_reward_nanomina set to a nonzero value (e.g. MAINNET_COINBASE_REWARD)
        //    - a single FeeTransfer
        //    - canonical = false, was_canonical = true => reversal scenario
        let test_fee_transfer = FeeTransfer {
            recipient: "B62qrecipientNONCANON".to_string(),
            fee_nanomina: 50_000_000_000, // e.g. 50 Mina
        };

        let test_block = MainnetBlockPayload {
            height: 200,
            state_hash: "hash_non_canonical_fee".to_string(),
            previous_state_hash: "prev_hash".to_string(),
            last_vrf_output: "vrf_output_str".to_string(),
            user_command_count: 0,
            internal_command_count: 0,
            user_commands: vec![], // no user commands
            snark_work_count: 0,
            snark_work: vec![],
            timestamp: 123456789,
            coinbase_receiver: "B62qcoinbaseReceiver".to_string(),
            coinbase_reward_nanomina: MAINNET_COINBASE_REWARD, // let's say 72000000000 or your real constant
            global_slot_since_genesis: 999,
            fee_transfer_via_coinbase: None,
            fee_transfers: vec![test_fee_transfer],
            global_slot: 999,
        };

        let payload = CanonicalMainnetBlockPayload {
            block: test_block,
            canonical: false, // reversing the previously canonical block
            was_canonical: true,
        };

        // 7) Send the event
        actor_sender
            .send(Event {
                event_type: EventType::CanonicalMainnetBlock,
                payload: sonic_rs::to_string(&payload).unwrap(),
            })
            .await
            .expect("Failed to send event");

        // Wait for processing
        sleep(Duration::from_millis(200)).await;

        // 8) Read from the sink node
        let transactions = read_captured_transactions(&dag, sink_node_id).await;
        assert_eq!(transactions.len(), 1, "Expected exactly one DoubleEntryTransaction event in the sink");

        // 9) Parse the single DoubleEntryRecordPayload and fully verify
        let record_json = &transactions[0];
        let record: DoubleEntryRecordPayload = sonic_rs::from_str(record_json).expect("Failed to parse DoubleEntryRecordPayload");

        assert_eq!(record.height, 200);
        assert_eq!(record.state_hash, "hash_non_canonical_fee");
        assert_eq!(record.ledger_destination, LedgerDestination::BlockchainLedger);

        // Because we have 1 fee transfer + coinbase, we expect 2 pairs of entries => total 2 LHS, 2 RHS
        // One pair for the fee transfer, one pair for the coinbase. Both reversed.
        assert_eq!(
            record.lhs.len(),
            2,
            "Expected exactly 2 entries on LHS (reversed coinbase + reversed fee transfer)"
        );
        assert_eq!(
            record.rhs.len(),
            2,
            "Expected exactly 2 entries on RHS (reversed coinbase + reversed fee transfer)"
        );

        // ----------------------
        // Check LHS[0] => reversed fee transfer debit => becomes a credit
        // Check RHS[0] => reversed fee transfer credit => becomes a debit
        // Because you can't be sure which partial method got appended first in the list,
        // you may need to do a partial check or find them by matching `transfer_type`.
        // We'll assume the fee transfer was appended first, coinbase second, if your code
        // processes them in that order.
        // ----------------------

        // Fee Transfer
        let lhs_fee = &record.lhs[0];
        let rhs_fee = &record.rhs[0];

        assert_eq!(lhs_fee.transfer_type, "FeeTransfer");
        // Because it's non-canonical, we expect the pool to be a "Credit" in LHS
        assert_eq!(lhs_fee.entry_type, AccountingEntryType::Credit);
        assert_eq!(lhs_fee.account, "BlockRewardPool#hash_non_canonical_fee");

        // Meanwhile, the recipient is a "Debit" in RHS
        assert_eq!(rhs_fee.transfer_type, "FeeTransfer");
        assert_eq!(rhs_fee.entry_type, AccountingEntryType::Debit);
        assert_eq!(rhs_fee.account, "B62qrecipientNONCANON");

        // check amounts
        assert_eq!(lhs_fee.amount_nanomina, 50_000_000_000);
        assert_eq!(rhs_fee.amount_nanomina, 50_000_000_000);

        // ----------------------
        // Check LHS[1] => reversed coinbase => originally a debit => now a credit
        // Check RHS[1] => reversed coinbase => originally a credit => now a debit
        // ----------------------

        let lhs_coinbase = &record.lhs[1];
        let rhs_coinbase = &record.rhs[1];

        assert_eq!(lhs_coinbase.transfer_type, "Coinbase");
        assert_eq!(lhs_coinbase.entry_type, AccountingEntryType::Credit);
        assert_eq!(lhs_coinbase.account, "MinaCoinbasePayment#hash_non_canonical_fee");

        assert_eq!(rhs_coinbase.transfer_type, "Coinbase");
        assert_eq!(rhs_coinbase.entry_type, AccountingEntryType::Debit);
        assert_eq!(rhs_coinbase.account, "B62qcoinbaseReceiver");

        // check amounts match your MAINNET_COINBASE_REWARD
        assert_eq!(lhs_coinbase.amount_nanomina, MAINNET_COINBASE_REWARD);
        assert_eq!(rhs_coinbase.amount_nanomina, MAINNET_COINBASE_REWARD);
    }

    #[tokio::test]
    async fn test_canonical_fee_transfer_via_coinbase() {
        // 1) Create the shutdown signal
        let (shutdown_tx, _shutdown_rx) = tokio::sync::watch::channel(false);

        // 2) Build the ActorDAG
        let mut dag = ActorDAG::new();

        // 3) Create the AccountingActor (root)
        let accounting_actor = AccountingActor::create_actor().await;
        let actor_id = accounting_actor.id();
        // set_root => returns a Sender<Event>
        let actor_sender = dag.set_root(accounting_actor);

        // 4) Create the sink node, add to DAG, link from AccountingActor
        let sink_node = create_double_entry_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, sink_node_id);

        // 5) Wrap + spawn
        let dag = std::sync::Arc::new(tokio::sync::Mutex::new(dag));
        tokio::spawn({
            let dag = std::sync::Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 6) Construct a MainnetBlockPayload that has `fee_transfer_via_coinbase = Some(...)` We'll keep everything else minimal. For demonstration, set
        //    coinbase_reward_nanomina = 0 to focus on FeeTransferViaCoinbase specifically.
        let test_xfer = FeeTransferViaCoinbase {
            receiver: "B62TestReceiverOfFTVC".to_string(),
            fee_nanomina: 1_500_000_000, // e.g., 1.5 Mina
        };

        let test_block = MainnetBlockPayload {
            height: 3000,
            state_hash: "hash_canonical_fee_via_coinbase".to_string(),
            previous_state_hash: "prev_hash".to_string(),
            last_vrf_output: "vrf_output_example".to_string(),
            user_command_count: 0,
            internal_command_count: 0,
            user_commands: vec![], // no user commands
            snark_work_count: 0,
            snark_work: vec![],
            timestamp: 123456789,
            coinbase_receiver: "B62qSomeCoinbaseReceiver".to_string(),
            coinbase_reward_nanomina: 0, // no direct coinbase in this test
            global_slot_since_genesis: 700,
            fee_transfer_via_coinbase: Some(vec![test_xfer]),
            fee_transfers: vec![], // no direct FeeTransfer
            global_slot: 700,
        };

        let payload = CanonicalMainnetBlockPayload {
            block: test_block,
            canonical: true, // canonical
            was_canonical: false,
        };

        // 7) Send the event
        actor_sender
            .send(Event {
                event_type: EventType::CanonicalMainnetBlock,
                payload: sonic_rs::to_string(&payload).unwrap(),
            })
            .await
            .expect("Failed to send canonical fee_transfer_via_coinbase event");

        // Wait for processing
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // 8) Read from the sink node
        let transactions = read_captured_transactions(&dag, sink_node_id).await;
        assert_eq!(
            transactions.len(),
            1,
            "Expected exactly one DoubleEntryTransaction event for the canonical block"
        );

        // 9) Parse the DoubleEntryRecordPayload and verify
        let record_json = &transactions[0];
        let record: DoubleEntryRecordPayload = sonic_rs::from_str(record_json).expect("Failed to parse DoubleEntryRecordPayload");

        assert_eq!(record.height, 3000);
        assert_eq!(record.state_hash, "hash_canonical_fee_via_coinbase");
        assert_eq!(record.ledger_destination, LedgerDestination::BlockchainLedger);

        // process_fee_transfer_via_coinbase typically yields 2 LHS + 2 RHS (the "BlockRewardPool" side
        // and the "FeeTransferViaCoinbase" side). For a single xfer, we expect 2 pairs => 2 LHS, 2 RHS
        assert_eq!(record.lhs.len(), 3, "Should have 2 reversed debit/credit pairs on LHS (plus coinbase)");
        assert_eq!(record.rhs.len(), 3, "Should have 2 reversed debit/credit pairs on RHS (plus coinbase)");

        // Minimal checks: the first LHS should be a debit from coinbase_receiver => block reward pool
        let lhs_0 = &record.lhs[0];
        let rhs_0 = &record.rhs[0];

        assert_eq!(lhs_0.entry_type, AccountingEntryType::Debit);
        assert!(
            lhs_0.transfer_type.contains("BlockRewardPool"),
            "Expected transfer_type to match 'BlockRewardPool'"
        );
        assert_eq!(lhs_0.account, "B62qSomeCoinbaseReceiver");
        assert_eq!(lhs_0.amount_nanomina, 1_500_000_000);

        assert_eq!(rhs_0.entry_type, AccountingEntryType::Credit);
        assert!(
            rhs_0.transfer_type.contains("BlockRewardPool"),
            "Expected transfer_type to match 'BlockRewardPool'"
        );
        assert_eq!(rhs_0.account, "BlockRewardPool#hash_canonical_fee_via_coinbase");
        assert_eq!(rhs_0.amount_nanomina, 1_500_000_000);

        // Second LHS => the "FeeTransferViaCoinbase" side (BlockRewardPool -> final recipient)
        let lhs_1 = &record.lhs[1];
        let rhs_1 = &record.rhs[1];

        assert_eq!(lhs_1.entry_type, AccountingEntryType::Debit);
        assert_eq!(lhs_1.transfer_type, "FeeTransferViaCoinbase");
        assert_eq!(lhs_1.account, "BlockRewardPool#hash_canonical_fee_via_coinbase");
        assert_eq!(lhs_1.amount_nanomina, 1_500_000_000);

        assert_eq!(rhs_1.entry_type, AccountingEntryType::Credit);
        assert_eq!(rhs_1.transfer_type, "FeeTransferViaCoinbase");
        assert_eq!(rhs_1.account, "B62TestReceiverOfFTVC");
        assert_eq!(rhs_1.amount_nanomina, 1_500_000_000);

        // 10) Shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_non_canonical_fee_transfer_via_coinbase() {
        // 1) Create the shutdown signal
        let (_shutdown_tx, _shutdown_rx) = tokio::sync::watch::channel(false);

        // 2) Build the ActorDAG
        let mut dag = ActorDAG::new();

        // 3) Create the AccountingActor (root)
        let accounting_actor = AccountingActor::create_actor().await;
        let actor_id = accounting_actor.id();
        let actor_sender = dag.set_root(accounting_actor);

        // 4) Create the sink node, add to DAG, link from AccountingActor
        let sink_node = create_double_entry_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, sink_node_id);

        // 5) Wrap + spawn
        let dag = std::sync::Arc::new(tokio::sync::Mutex::new(dag));
        tokio::spawn({
            let dag = std::sync::Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 6) Construct a block payload with 1 FeeTransferViaCoinbase, and canonical=false + was_canonical=true
        let test_xfer = FeeTransferViaCoinbase {
            receiver: "B62TestReceiverOfFTVC".to_string(),
            fee_nanomina: 2_345_678_000, // e.g. 2.345678 Mina
        };

        let test_block = MainnetBlockPayload {
            height: 5555,
            state_hash: "hash_non_canon_fee_via_coinbase".to_string(),
            previous_state_hash: "prev_state_hash".to_string(),
            last_vrf_output: "vrf_output_example2".to_string(),
            user_command_count: 0,
            internal_command_count: 0,
            user_commands: vec![], // none
            snark_work_count: 0,
            snark_work: vec![],
            timestamp: 987654321,
            coinbase_receiver: "B62qSomeCoinbaseReceiver2".to_string(),
            coinbase_reward_nanomina: 0, // ignoring coinbase in this test
            global_slot_since_genesis: 9999,
            fee_transfer_via_coinbase: Some(vec![test_xfer]),
            fee_transfers: vec![], // none
            global_slot: 9999,
        };

        let payload = CanonicalMainnetBlockPayload {
            block: test_block,
            canonical: false,    // reversing scenario
            was_canonical: true, // it was canonical before
        };

        // 7) Send the event
        actor_sender
            .send(Event {
                event_type: EventType::CanonicalMainnetBlock,
                payload: sonic_rs::to_string(&payload).unwrap(),
            })
            .await
            .expect("Failed to send non-canonical fee_transfer_via_coinbase event");

        // Wait for processing
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // 8) Check sink
        let transactions = read_captured_transactions(&dag, sink_node_id).await;
        assert_eq!(
            transactions.len(),
            1,
            "Expected exactly one DoubleEntryTransaction event for the non-canonical block"
        );

        // 9) Parse & verify
        let record_json = &transactions[0];
        let record: DoubleEntryRecordPayload = sonic_rs::from_str(record_json).expect("Failed to parse DoubleEntryRecordPayload");

        assert_eq!(record.height, 5555);
        assert_eq!(record.state_hash, "hash_non_canon_fee_via_coinbase");
        assert_eq!(record.ledger_destination, LedgerDestination::BlockchainLedger);

        // For a single FeeTransferViaCoinbase, we expect 2 pairs => 2 LHS and 2 RHS.
        // Because it's non-canonical, everything is reversed from the “normal” canonical scenario.
        assert_eq!(record.lhs.len(), 3, "Should have 2 reversed debit/credit pairs on LHS (plus coinbase)");
        assert_eq!(record.rhs.len(), 3, "Should have 2 reversed debit/credit pairs on RHS (plus coinbase)");

        // LHS[0]/RHS[0]: reversed “BlockRewardPool” side
        let lhs_0 = &record.lhs[0];
        let rhs_0 = &record.rhs[0];

        // Typically in canonical mode => LHS: Debit from coinbaseReceiver, RHS: Credit reward pool
        // Now reversed => LHS: Credit reward pool, RHS: Debit coinbaseReceiver
        assert_eq!(lhs_0.transfer_type, "BlockRewardPool");
        assert_eq!(lhs_0.entry_type, AccountingEntryType::Credit);
        assert_eq!(lhs_0.account, "B62qSomeCoinbaseReceiver2");
        assert_eq!(lhs_0.amount_nanomina, 2_345_678_000);

        assert_eq!(rhs_0.transfer_type, "BlockRewardPool");
        assert_eq!(rhs_0.entry_type, AccountingEntryType::Debit);
        assert_eq!(rhs_0.account, "BlockRewardPool#hash_non_canon_fee_via_coinbase");
        assert_eq!(rhs_0.amount_nanomina, 2_345_678_000);

        // LHS[1]/RHS[1]: reversed “FeeTransferViaCoinbase” side
        let lhs_1 = &record.lhs[1];
        let rhs_1 = &record.rhs[1];

        // Canonical => LHS: Debit from “BlockRewardPool#…”, RHS: Credit final receiver
        // Non-canonical => swapped
        assert_eq!(lhs_1.transfer_type, "FeeTransferViaCoinbase");
        assert_eq!(lhs_1.entry_type, AccountingEntryType::Credit);
        assert_eq!(lhs_1.account, "BlockRewardPool#hash_non_canon_fee_via_coinbase");
        assert_eq!(lhs_1.amount_nanomina, 2_345_678_000);

        assert_eq!(rhs_1.transfer_type, "FeeTransferViaCoinbase");
        assert_eq!(rhs_1.entry_type, AccountingEntryType::Debit);
        assert_eq!(rhs_1.account, "B62TestReceiverOfFTVC");
        assert_eq!(rhs_1.amount_nanomina, 2_345_678_000);
    }

    #[tokio::test]
    async fn test_canonical_coinbase_only() {
        // 1) Create the shutdown signal
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2) Build the ActorDAG
        let mut dag = ActorDAG::new();

        // 3) Create the AccountingActor (root)
        let accounting_actor = AccountingActor::create_actor().await;
        let actor_id = accounting_actor.id();
        let actor_sender = dag.set_root(accounting_actor);

        // 4) Create the sink node, add to DAG, link from AccountingActor
        let sink_node = create_double_entry_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, sink_node_id);

        // 5) Wrap + spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 6) Build a MainnetBlockPayload that has only coinbase reward (nonzero), and canonical=true, was_canonical=false
        let test_block = MainnetBlockPayload {
            height: 1001,
            state_hash: "hash_canonical_coinbase_only".to_string(),
            previous_state_hash: "some_prev_hash".to_string(),
            last_vrf_output: "vrf_output_here".to_string(),
            user_command_count: 0,
            internal_command_count: 0,
            user_commands: vec![],
            snark_work_count: 0,
            snark_work: vec![],
            timestamp: 123456,
            coinbase_receiver: "B62qCoinbaseReceiverOnly".to_string(),
            coinbase_reward_nanomina: 99_000_000_000, // e.g. 99 Mina as reward
            global_slot_since_genesis: 1001,
            fee_transfer_via_coinbase: None,
            fee_transfers: vec![],
            global_slot: 1001,
        };

        let payload = CanonicalMainnetBlockPayload {
            block: test_block,
            canonical: true, // canonical
            was_canonical: false,
        };

        // 7) Send the event
        actor_sender
            .send(Event {
                event_type: EventType::CanonicalMainnetBlock,
                payload: sonic_rs::to_string(&payload).unwrap(),
            })
            .await
            .expect("Failed to send canonical coinbase-only event");

        // Wait a bit
        sleep(Duration::from_millis(200)).await;

        // 8) Read from the sink
        let transactions = read_captured_transactions(&dag, sink_node_id).await;
        assert_eq!(transactions.len(), 1, "Expected exactly 1 DoubleEntryTransaction for canonical coinbase-only");

        // 9) Parse the DoubleEntryRecordPayload and verify
        let record: DoubleEntryRecordPayload = sonic_rs::from_str(&transactions[0]).expect("Failed to parse DoubleEntryRecordPayload");

        assert_eq!(record.height, 1001);
        assert_eq!(record.state_hash, "hash_canonical_coinbase_only");
        assert_eq!(record.ledger_destination, LedgerDestination::BlockchainLedger);

        // With only coinbase, we expect 1 LHS + 1 RHS for a canonical scenario
        assert_eq!(record.lhs.len(), 1, "Expected exactly 1 debit from coinbase");
        assert_eq!(record.rhs.len(), 1, "Expected exactly 1 credit for coinbase");

        let lhs_0 = &record.lhs[0];
        let rhs_0 = &record.rhs[0];

        // Canonical coinbase => LHS: Debit from MinaCoinbasePayment#[state_hash],
        //                      RHS: Credit coinbase receiver
        assert_eq!(lhs_0.transfer_type, "Coinbase");
        assert_eq!(lhs_0.entry_type, AccountingEntryType::Debit);
        assert_eq!(lhs_0.account, "MinaCoinbasePayment#hash_canonical_coinbase_only");
        assert_eq!(lhs_0.amount_nanomina, 99_000_000_000);

        assert_eq!(rhs_0.transfer_type, "Coinbase");
        assert_eq!(rhs_0.entry_type, AccountingEntryType::Credit);
        assert_eq!(rhs_0.account, "B62qCoinbaseReceiverOnly");
        assert_eq!(rhs_0.amount_nanomina, 99_000_000_000);

        // 10) Shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_non_canonical_coinbase_only() {
        // 1) Create the shutdown signal
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2) Build the ActorDAG
        let mut dag = ActorDAG::new();

        // 3) Create the AccountingActor (root)
        let accounting_actor = AccountingActor::create_actor().await;
        let actor_id = accounting_actor.id();
        let actor_sender = dag.set_root(accounting_actor);

        // 4) Create the sink node, add to DAG, link from AccountingActor
        let sink_node = create_double_entry_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, sink_node_id);

        // 5) Wrap + spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 6) Build a MainnetBlockPayload that has only coinbase reward, but canonical=false, was_canonical=true => reversed coinbase
        let test_block = MainnetBlockPayload {
            height: 2222,
            state_hash: "hash_non_canonical_coinbase_only".to_string(),
            previous_state_hash: "some_prev_hash2".to_string(),
            last_vrf_output: "vrf_output_other".to_string(),
            user_command_count: 0,
            internal_command_count: 0,
            user_commands: vec![],
            snark_work_count: 0,
            snark_work: vec![],
            timestamp: 987654,
            coinbase_receiver: "B62qNonCanonCoinbaseReceiver".to_string(),
            coinbase_reward_nanomina: 88_000_000_000, // e.g. 88 Mina
            global_slot_since_genesis: 2222,
            fee_transfer_via_coinbase: None,
            fee_transfers: vec![],
            global_slot: 2222,
        };

        let payload = CanonicalMainnetBlockPayload {
            block: test_block,
            canonical: false,    // reversed
            was_canonical: true, // previously canonical
        };

        // 7) Send the event
        actor_sender
            .send(Event {
                event_type: EventType::CanonicalMainnetBlock,
                payload: sonic_rs::to_string(&payload).unwrap(),
            })
            .await
            .expect("Failed to send non-canonical coinbase-only event");

        // Wait a bit
        sleep(Duration::from_millis(200)).await;

        // 8) Read from the sink
        let transactions = read_captured_transactions(&dag, sink_node_id).await;
        assert_eq!(
            transactions.len(),
            1,
            "Expected exactly 1 DoubleEntryTransaction for non-canonical coinbase-only"
        );

        // 9) Parse & verify
        let record: DoubleEntryRecordPayload = sonic_rs::from_str(&transactions[0]).expect("Failed to parse DoubleEntryRecordPayload");

        assert_eq!(record.height, 2222);
        assert_eq!(record.state_hash, "hash_non_canonical_coinbase_only");
        assert_eq!(record.ledger_destination, LedgerDestination::BlockchainLedger);

        // Non-canonical => 1 LHS + 1 RHS, reversed
        assert_eq!(record.lhs.len(), 1, "Expected exactly 1 reversed debit/credit in LHS");
        assert_eq!(record.rhs.len(), 1, "Expected exactly 1 reversed debit/credit in RHS");

        let lhs_0 = &record.lhs[0];
        let rhs_0 = &record.rhs[0];

        // In canonical, coinbase => LHS: Debit "MinaCoinbasePayment#...", RHS: Credit coinbaseReceiver
        // Now reversed => LHS is a credit entry for "MinaCoinbasePayment#...", RHS is a debit to coinbaseReceiver
        assert_eq!(lhs_0.transfer_type, "Coinbase");
        assert_eq!(lhs_0.entry_type, AccountingEntryType::Credit);
        assert_eq!(lhs_0.account, "MinaCoinbasePayment#hash_non_canonical_coinbase_only");
        assert_eq!(lhs_0.amount_nanomina, 88_000_000_000);

        assert_eq!(rhs_0.transfer_type, "Coinbase");
        assert_eq!(rhs_0.entry_type, AccountingEntryType::Debit);
        assert_eq!(rhs_0.account, "B62qNonCanonCoinbaseReceiver");
        assert_eq!(rhs_0.amount_nanomina, 88_000_000_000);

        // 10) Shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_canonical_user_command_payment() {
        // 1) Create shutdown signal
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2) Build the ActorDAG
        let mut dag = ActorDAG::new();

        // 3) Create the AccountingActor (root)
        let accounting_actor = AccountingActor::create_actor().await;
        let actor_id = accounting_actor.id();
        let actor_sender = dag.set_root(accounting_actor);

        // 4) Create and link the sink node
        let sink_node = create_double_entry_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, sink_node_id);

        // 5) Wrap + spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 6) Build a single user command (Applied Payment):
        //    - sender pays 100_000_000_000 to receiver
        //    - fee payer = sender, fee_nanomina = 2_000_000_000
        //    - canonical = true => normal arrangement
        let test_command = CommandSummary {
            sender: "B62qSenderCmd".to_string(),
            receiver: "B62qReceiverCmd".to_string(),
            fee_payer: "B62qFeePayerCmd".to_string(),
            amount_nanomina: 100_000_000_000,
            fee_nanomina: 2_000_000_000,
            txn_type: crate::event_sourcing::models::CommandType::Payment,
            status: crate::event_sourcing::models::CommandStatus::Applied,
            // other fields like nonce/memo omitted for brevity
            ..Default::default()
        };

        let test_block = MainnetBlockPayload {
            height: 500,
            state_hash: "hash_cmd_payment_canonical".to_string(),
            previous_state_hash: "prev_cmd_hash".to_string(),
            last_vrf_output: "vrf_output_cmd".to_string(),
            user_command_count: 1,
            internal_command_count: 0,
            user_commands: vec![test_command],
            snark_work_count: 0,
            snark_work: vec![],
            timestamp: 111_222_333,
            coinbase_receiver: "B62qNoCoinbase".to_string(),
            coinbase_reward_nanomina: 0,
            global_slot_since_genesis: 100,
            fee_transfer_via_coinbase: None,
            fee_transfers: vec![],
            global_slot: 100,
        };

        let payload = CanonicalMainnetBlockPayload {
            block: test_block,
            canonical: true, // canonical
            was_canonical: false,
        };

        // 7) Send the event
        actor_sender
            .send(Event {
                event_type: EventType::CanonicalMainnetBlock,
                payload: sonic_rs::to_string(&payload).unwrap(),
            })
            .await
            .expect("Failed to send canonical user_command event");

        // Wait a bit
        sleep(Duration::from_millis(200)).await;

        // 8) Read from sink
        let transactions = read_captured_transactions(&dag, sink_node_id).await;
        assert_eq!(transactions.len(), 1, "Expected 1 DoubleEntryTransaction for canonical user command");

        // 9) Parse & check
        let record: DoubleEntryRecordPayload = sonic_rs::from_str(&transactions[0]).expect("Failed to parse DoubleEntryRecordPayload");
        assert_eq!(record.height, 500);
        assert_eq!(record.state_hash, "hash_cmd_payment_canonical");
        assert_eq!(record.ledger_destination, LedgerDestination::BlockchainLedger);

        // The partial logic for user commands says:
        //   - If Applied + not StakeDelegation => we do sender->receiver
        //   - We always do fee payer->block reward pool
        // So we expect 2 pairs => 2 LHS, 2 RHS
        assert_eq!(record.lhs.len(), 3, "Expected 2 debits: payment, fee (+ FTVC)");
        assert_eq!(record.rhs.len(), 3, "Expected 2 credits: payment, fee (+ FTVC)");

        // Payment pair:
        //   LHS => Debit from sender
        //   RHS => Credit to receiver
        let lhs_payment = &record.lhs[0];
        let rhs_payment = &record.rhs[0];

        assert_eq!(lhs_payment.transfer_type, "Payment");
        assert_eq!(lhs_payment.entry_type, AccountingEntryType::Debit);
        assert_eq!(lhs_payment.account, "B62qSenderCmd");
        assert_eq!(lhs_payment.amount_nanomina, 100_000_000_000);

        assert_eq!(rhs_payment.transfer_type, "Payment");
        assert_eq!(rhs_payment.entry_type, AccountingEntryType::Credit);
        assert_eq!(rhs_payment.account, "B62qReceiverCmd");
        assert_eq!(rhs_payment.amount_nanomina, 100_000_000_000);

        // Fee pair:
        //   LHS => Debit from fee_payer
        //   RHS => Credit to BlockRewardPool#[state_hash]
        let lhs_fee = &record.lhs[1];
        let rhs_fee = &record.rhs[1];

        assert_eq!(lhs_fee.transfer_type, "BlockRewardPool");
        assert_eq!(lhs_fee.entry_type, AccountingEntryType::Debit);
        assert_eq!(lhs_fee.account, "B62qFeePayerCmd");
        assert_eq!(lhs_fee.amount_nanomina, 2_000_000_000);

        assert_eq!(rhs_fee.transfer_type, "BlockRewardPool");
        assert_eq!(rhs_fee.entry_type, AccountingEntryType::Credit);
        assert_eq!(rhs_fee.account, "BlockRewardPool#hash_cmd_payment_canonical");
        assert_eq!(rhs_fee.amount_nanomina, 2_000_000_000);

        // 10) Shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_non_canonical_user_command_payment() {
        // 1) Create shutdown
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2) DAG
        let mut dag = ActorDAG::new();

        // 3) Root
        let accounting_actor = AccountingActor::create_actor().await;
        let actor_id = accounting_actor.id();
        let actor_sender = dag.set_root(accounting_actor);

        // 4) Sink
        let sink_node = create_double_entry_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, sink_node_id);

        // 5) spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 6) Non-canonical scenario => canonical=false, was_canonical=true 1 user command => Payment, status=Applied => reversed
        let test_command = CommandSummary {
            sender: "B62qSenderCmd".to_string(),
            receiver: "B62qReceiverCmd".to_string(),
            fee_payer: "B62qFeePayerCmd".to_string(),
            amount_nanomina: 77_000_000_000,
            fee_nanomina: 3_000_000_000,
            txn_type: crate::event_sourcing::models::CommandType::Payment,
            status: crate::event_sourcing::models::CommandStatus::Applied,
            ..Default::default()
        };

        let test_block = MainnetBlockPayload {
            height: 501,
            state_hash: "hash_cmd_payment_noncan".to_string(),
            previous_state_hash: "prev_cmd_hash_noncan".to_string(),
            last_vrf_output: "vrf_output_cmd_noncan".to_string(),
            user_command_count: 1,
            internal_command_count: 0,
            user_commands: vec![test_command],
            snark_work_count: 0,
            snark_work: vec![],
            timestamp: 111_222_444,
            coinbase_receiver: "B62qNoCoinbaseNoncan".to_string(),
            coinbase_reward_nanomina: 0,
            global_slot_since_genesis: 101,
            fee_transfer_via_coinbase: None,
            fee_transfers: vec![],
            global_slot: 101,
        };

        let payload = CanonicalMainnetBlockPayload {
            block: test_block,
            canonical: false,    // reversed
            was_canonical: true, // was canonical before
        };

        // 7) Send
        actor_sender
            .send(Event {
                event_type: EventType::CanonicalMainnetBlock,
                payload: sonic_rs::to_string(&payload).unwrap(),
            })
            .await
            .expect("Failed to send non-canonical user_command event");

        // wait
        sleep(Duration::from_millis(200)).await;

        // 8) read sink
        let transactions = read_captured_transactions(&dag, sink_node_id).await;
        assert_eq!(transactions.len(), 1, "Expected 1 DoubleEntryTransaction for non-canonical user command");

        // 9) parse
        let record: DoubleEntryRecordPayload = sonic_rs::from_str(&transactions[0]).expect("Failed to parse DoubleEntryRecordPayload");
        assert_eq!(record.height, 501);
        assert_eq!(record.state_hash, "hash_cmd_payment_noncan");
        assert_eq!(record.ledger_destination, LedgerDestination::BlockchainLedger);

        // We expect 2 pairs => reversed
        assert_eq!(record.lhs.len(), 3, "Expected 2 debits: payment, fee (+ FTVC)");
        assert_eq!(record.rhs.len(), 3, "Expected 2 credits: payment, fee (+ FTVC)");

        // Payment reversal
        // canonical => LHS: Debit from sender, RHS: Credit to receiver
        // reversed => LHS: Credit to sender, RHS: Debit from receiver
        let lhs_pay = &record.lhs[0];
        let rhs_pay = &record.rhs[0];

        assert_eq!(lhs_pay.transfer_type, "Payment");
        assert_eq!(lhs_pay.entry_type, AccountingEntryType::Credit);
        assert_eq!(lhs_pay.account, "B62qSenderCmd");
        assert_eq!(lhs_pay.amount_nanomina, 77_000_000_000);

        assert_eq!(rhs_pay.transfer_type, "Payment");
        assert_eq!(rhs_pay.entry_type, AccountingEntryType::Debit);
        assert_eq!(rhs_pay.account, "B62qReceiverCmd");
        assert_eq!(rhs_pay.amount_nanomina, 77_000_000_000);

        // Fee reversal
        // canonical => LHS: Debit from fee_payer, RHS: Credit to BlockRewardPool#[state_hash]
        // reversed => LHS: Credit to fee_payer, RHS: Debit from reward pool
        let lhs_fee = &record.lhs[1];
        let rhs_fee = &record.rhs[1];

        assert_eq!(lhs_fee.transfer_type, "BlockRewardPool");
        assert_eq!(lhs_fee.entry_type, AccountingEntryType::Credit);
        assert_eq!(lhs_fee.account, "B62qFeePayerCmd");
        assert_eq!(lhs_fee.amount_nanomina, 3_000_000_000);

        assert_eq!(rhs_fee.transfer_type, "BlockRewardPool");
        assert_eq!(rhs_fee.entry_type, AccountingEntryType::Debit);
        assert_eq!(rhs_fee.account, "BlockRewardPool#hash_cmd_payment_noncan");
        assert_eq!(rhs_fee.amount_nanomina, 3_000_000_000);

        // 10) shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_canonical_user_command_failed_payment() {
        // 1) Create shutdown signal
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2) Build the ActorDAG
        let mut dag = ActorDAG::new();

        // 3) Create the AccountingActor (root)
        let accounting_actor = AccountingActor::create_actor().await;
        let actor_id = accounting_actor.id();
        let actor_sender = dag.set_root(accounting_actor);

        // 4) Create and link the sink node
        let sink_node = create_double_entry_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, sink_node_id);

        // 5) Wrap + spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 6) Build a user command with status = Failed, txn_type = Payment => no sender→receiver pair, but we do have the fee pair.
        let test_command = CommandSummary {
            sender: "B62qSenderFailed".to_string(),
            receiver: "B62qReceiverFailed".to_string(),
            fee_payer: "B62qFeePayerFailed".to_string(),
            amount_nanomina: 55_000_000_000, // would be the payment, but it's ignored for "Failed"
            fee_nanomina: 1_000_000_000,
            txn_type: crate::event_sourcing::models::CommandType::Payment,
            status: crate::event_sourcing::models::CommandStatus::Failed, // not Applied
            ..Default::default()
        };

        let test_block = MainnetBlockPayload {
            height: 600,
            state_hash: "hash_cmd_failed_payment".to_string(),
            previous_state_hash: "prev_cmd_hash_failed".to_string(),
            last_vrf_output: "vrf_output_failed".to_string(),
            user_command_count: 1,
            internal_command_count: 0,
            user_commands: vec![test_command],
            snark_work_count: 0,
            snark_work: vec![],
            timestamp: 999_888_777,
            coinbase_receiver: "B62qNoCoinbaseFailed".to_string(),
            coinbase_reward_nanomina: 0,
            global_slot_since_genesis: 202,
            fee_transfer_via_coinbase: None,
            fee_transfers: vec![],
            global_slot: 202,
        };

        let payload = CanonicalMainnetBlockPayload {
            block: test_block,
            canonical: true,
            was_canonical: false,
        };

        // 7) Send the event
        actor_sender
            .send(Event {
                event_type: EventType::CanonicalMainnetBlock,
                payload: sonic_rs::to_string(&payload).unwrap(),
            })
            .await
            .expect("Failed to send failed user_command event");

        // 8) Wait a bit
        sleep(Duration::from_millis(200)).await;

        // 9) Read from sink
        let transactions = read_captured_transactions(&dag, sink_node_id).await;
        assert_eq!(transactions.len(), 1, "Expected 1 DoubleEntryTransaction for failed user command");

        // 10) Parse & check
        let record: DoubleEntryRecordPayload = sonic_rs::from_str(&transactions[0]).expect("Failed to parse DoubleEntryRecordPayload");

        assert_eq!(record.height, 600);
        assert_eq!(record.state_hash, "hash_cmd_failed_payment");
        assert_eq!(record.ledger_destination, LedgerDestination::BlockchainLedger);

        // Because status=Failed => NO sender→receiver pair, but we STILL do fee_payer→BlockRewardPool
        // => exactly 1 pair => 1 LHS, 1 RHS
        assert_eq!(record.lhs.len(), 2, "Only 1 debit: the fee (+ FTVC)");
        assert_eq!(record.rhs.len(), 2, "Only 1 credit: the fee (+ FTVC)");

        // Check that single fee pair
        let lhs_fee = &record.lhs[0];
        let rhs_fee = &record.rhs[0];

        assert_eq!(lhs_fee.transfer_type, "BlockRewardPool");
        assert_eq!(lhs_fee.entry_type, AccountingEntryType::Debit);
        assert_eq!(lhs_fee.account, "B62qFeePayerFailed");
        assert_eq!(lhs_fee.amount_nanomina, 1_000_000_000);

        assert_eq!(rhs_fee.transfer_type, "BlockRewardPool");
        assert_eq!(rhs_fee.entry_type, AccountingEntryType::Credit);
        assert_eq!(rhs_fee.account, "BlockRewardPool#hash_cmd_failed_payment");
        assert_eq!(rhs_fee.amount_nanomina, 1_000_000_000);

        // shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_canonical_user_command_stake_delegation() {
        // 1) Create shutdown
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);

        // 2) DAG
        let mut dag = ActorDAG::new();

        // 3) Root
        let accounting_actor = AccountingActor::create_actor().await;
        let actor_id = accounting_actor.id();
        let actor_sender = dag.set_root(accounting_actor);

        // 4) Sink
        let sink_node = create_double_entry_sink_node();
        let sink_node_id = &sink_node.id();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, sink_node_id);

        // 5) spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all().await;
            }
        });

        // 6) stake delegation => even if Applied, we do no sender->receiver pair, only fee pair
        let test_command = CommandSummary {
            sender: "B62qSenderStake".to_string(),
            receiver: "B62qReceiverStake".to_string(),
            fee_payer: "B62qFeePayerStake".to_string(),
            amount_nanomina: 999_000_000_000,
            fee_nanomina: 50_000_000_000,
            txn_type: crate::event_sourcing::models::CommandType::StakeDelegation,
            status: crate::event_sourcing::models::CommandStatus::Applied,
            ..Default::default()
        };

        let test_block = MainnetBlockPayload {
            height: 700,
            state_hash: "hash_cmd_stake_delegation".to_string(),
            previous_state_hash: "prev_cmd_hash_stake".to_string(),
            last_vrf_output: "vrf_output_stake".to_string(),
            user_command_count: 1,
            internal_command_count: 0,
            user_commands: vec![test_command],
            snark_work_count: 0,
            snark_work: vec![],
            timestamp: 222_333_444,
            coinbase_receiver: "B62qNoCoinbaseStake".to_string(),
            coinbase_reward_nanomina: 0,
            global_slot_since_genesis: 300,
            fee_transfer_via_coinbase: None,
            fee_transfers: vec![],
            global_slot: 300,
        };

        let payload = CanonicalMainnetBlockPayload {
            block: test_block,
            canonical: true, // normal arrangement
            was_canonical: false,
        };

        // 7) Send
        actor_sender
            .send(Event {
                event_type: EventType::CanonicalMainnetBlock,
                payload: sonic_rs::to_string(&payload).unwrap(),
            })
            .await
            .expect("Failed to send stake delegation user_command event");

        // wait
        sleep(Duration::from_millis(200)).await;

        // 8) read sink
        let transactions = read_captured_transactions(&dag, sink_node_id).await;
        assert_eq!(
            transactions.len(),
            1,
            "Expected 1 DoubleEntryTransaction for canonical stake delegation user command"
        );

        // 9) parse
        let record: DoubleEntryRecordPayload = sonic_rs::from_str(&transactions[0]).expect("Failed to parse DoubleEntryRecordPayload");
        assert_eq!(record.height, 700);
        assert_eq!(record.state_hash, "hash_cmd_stake_delegation");
        assert_eq!(record.ledger_destination, LedgerDestination::BlockchainLedger);

        // Because stake delegation => no sender->receiver pair, but we still do fee
        // => 1 pair => 1 LHS, 1 RHS
        assert_eq!(
            record.lhs.len(),
            2,
            "Should have exactly 1 debit entry (fee payer -> reward pool) for stake delegation (+ FTVC)"
        );
        assert_eq!(
            record.rhs.len(),
            2,
            "Should have exactly 1 credit entry (fee payer -> reward pool) for stake delegation (+ FTVC)"
        );

        let lhs_fee = &record.lhs[0];
        let rhs_fee = &record.rhs[0];

        assert_eq!(lhs_fee.transfer_type, "BlockRewardPool");
        assert_eq!(lhs_fee.entry_type, AccountingEntryType::Debit);
        assert_eq!(lhs_fee.account, "B62qFeePayerStake");
        assert_eq!(lhs_fee.amount_nanomina, 50_000_000_000);

        assert_eq!(rhs_fee.transfer_type, "BlockRewardPool");
        assert_eq!(rhs_fee.entry_type, AccountingEntryType::Credit);
        assert_eq!(rhs_fee.account, "BlockRewardPool#hash_cmd_stake_delegation");
        assert_eq!(rhs_fee.amount_nanomina, 50_000_000_000);

        // 10) shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }
}
