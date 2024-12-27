use crate::event_sourcing::{
    actor_dag::{ActorFactory, ActorNode, ActorNodeBuilder, ActorStore},
    events::{Event, EventType},
    models::{CommandSummary, CommandType, FeeTransfer, FeeTransferViaCoinbase, ZkAppCommandSummary},
    payloads::{
        AccountingEntry, AccountingEntryAccountType, AccountingEntryType, CanonicalBerkeleyBlockPayload, CanonicalMainnetBlockPayload,
        DoubleEntryRecordPayload, InternalCommandType, LedgerDestination,
    },
};

pub struct AccountingActor;

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
}

impl ActorFactory for AccountingActor {
    fn create_actor() -> ActorNode {
        ActorNodeBuilder::new("BlockCanonicityActor".to_string())
            .with_state(ActorStore::new())
            .with_processor(|event, _state, _requeue| {
                Box::pin(async move {
                    match event.event_type {
                        EventType::CanonicalMainnetBlock => {
                            // 1) Parse the block payload
                            let payload: CanonicalMainnetBlockPayload =
                                sonic_rs::from_str(&event.payload).expect("Failed to parse CanonicalMainnetBlockPayload");

                            // 2) If neither canonical nor was_canonical => no reversal or forward needed
                            if !payload.canonical && !payload.was_canonical {
                                return None;
                            }

                            // 3) Extract the underlying MainnetBlockPayload
                            let mainnet_block = payload.block;

                            // We'll collect **all** LHS (debit) and RHS (credit) entries
                            // in two vectors, then produce one single event at the end.
                            let mut total_lhs: Vec<AccountingEntry> = Vec::new();
                            let mut total_rhs: Vec<AccountingEntry> = Vec::new();

                            // ----- USER COMMANDS -----
                            for cmd in mainnet_block.user_commands {
                                let (lhs, rhs) = Self::process_user_command(&mainnet_block.state_hash, mainnet_block.timestamp, &cmd, payload.canonical).await;

                                // Merge them into the big lists
                                total_lhs.extend(lhs);
                                total_rhs.extend(rhs);
                            }

                            // ----- FEE TRANSFERS -----
                            for fee_transfer in mainnet_block.fee_transfers {
                                let (lhs, rhs) =
                                    Self::process_fee_transfer(&mainnet_block.state_hash, mainnet_block.timestamp, fee_transfer, payload.canonical).await;

                                total_lhs.extend(lhs);
                                total_rhs.extend(rhs);
                            }

                            // ----- FEE TRANSFER VIA COINBASE -----
                            if let Some(fee_via_coinbase) = mainnet_block.fee_transfer_via_coinbase {
                                for xfer in fee_via_coinbase {
                                    let (lhs, rhs) = Self::process_fee_transfer_via_coinbase(
                                        &mainnet_block.state_hash,
                                        mainnet_block.timestamp,
                                        &mainnet_block.coinbase_receiver,
                                        &xfer,
                                        payload.canonical,
                                    )
                                    .await;

                                    total_lhs.extend(lhs);
                                    total_rhs.extend(rhs);
                                }
                            }

                            // ----- COINBASE -----
                            let (lhs, rhs) = Self::process_coinbase(
                                &mainnet_block.state_hash,
                                mainnet_block.timestamp,
                                &mainnet_block.coinbase_receiver,
                                mainnet_block.coinbase_reward_nanomina,
                                payload.canonical,
                            )
                            .await;

                            total_lhs.extend(lhs);
                            total_rhs.extend(rhs);

                            // If we ended up with zero total LHS and RHS, produce no event
                            if total_lhs.is_empty() && total_rhs.is_empty() {
                                return None;
                            }

                            // Otherwise, produce one single DoubleEntryRecordPayload event
                            let record = DoubleEntryRecordPayload {
                                height: mainnet_block.height,
                                state_hash: mainnet_block.state_hash.clone(),
                                ledger_destination: LedgerDestination::BlockchainLedger,
                                lhs: total_lhs,
                                rhs: total_rhs,
                            };

                            // Runtime check to verify LHS and RHS balance
                            record.verify();

                            let new_event = Event {
                                event_type: EventType::DoubleEntryTransaction,
                                payload: sonic_rs::to_string(&record).unwrap(),
                            };

                            // Return a single-event vector
                            Some(vec![new_event])
                        }
                        EventType::CanonicalBerkeleyBlock => {
                            // 1) Parse the block payload
                            let payload: CanonicalBerkeleyBlockPayload =
                                sonic_rs::from_str(&event.payload).expect("Failed to parse CanonicalMainnetBlockPayload");

                            // 2) If neither canonical nor was_canonical => no reversal or forward needed
                            if !payload.canonical && !payload.was_canonical {
                                return None;
                            }

                            // 3) Extract the underlying BerkeleyBlockPayload
                            let berkeley_block = payload.block;

                            // We'll collect **all** LHS (debit) and RHS (credit) entries
                            // in two vectors, then produce one single event at the end.
                            let mut total_lhs: Vec<AccountingEntry> = Vec::new();
                            let mut total_rhs: Vec<AccountingEntry> = Vec::new();

                            // ----- USER COMMANDS -----
                            for cmd in berkeley_block.user_commands {
                                let (lhs, rhs) =
                                    Self::process_user_command(&berkeley_block.state_hash, berkeley_block.timestamp, &cmd, payload.canonical).await;

                                // Merge them into the big lists
                                total_lhs.extend(lhs);
                                total_rhs.extend(rhs);
                            }

                            // ----- ZK APP COMMANDS -----
                            for cmd in berkeley_block.zk_app_commands {
                                let (lhs, rhs) =
                                    Self::process_batch_zk_app_commands(&berkeley_block.state_hash, berkeley_block.timestamp, &cmd, payload.canonical).await;

                                // Merge them into the big lists
                                total_lhs.extend(lhs);
                                total_rhs.extend(rhs);
                            }

                            // ----- FEE TRANSFERS -----
                            for fee_transfer in berkeley_block.fee_transfers {
                                let (lhs, rhs) =
                                    Self::process_fee_transfer(&berkeley_block.state_hash, berkeley_block.timestamp, fee_transfer, payload.canonical).await;

                                total_lhs.extend(lhs);
                                total_rhs.extend(rhs);
                            }

                            // ----- FEE TRANSFER VIA COINBASE -----
                            if let Some(fee_via_coinbase) = berkeley_block.fee_transfer_via_coinbase {
                                for xfer in fee_via_coinbase {
                                    let (lhs, rhs) = Self::process_fee_transfer_via_coinbase(
                                        &berkeley_block.state_hash,
                                        berkeley_block.timestamp,
                                        &berkeley_block.coinbase_receiver,
                                        &xfer,
                                        payload.canonical,
                                    )
                                    .await;

                                    total_lhs.extend(lhs);
                                    total_rhs.extend(rhs);
                                }
                            }

                            // ----- COINBASE -----
                            let (lhs, rhs) = Self::process_coinbase(
                                &berkeley_block.state_hash,
                                berkeley_block.timestamp,
                                &berkeley_block.coinbase_receiver,
                                berkeley_block.coinbase_reward_nanomina,
                                payload.canonical,
                            )
                            .await;

                            total_lhs.extend(lhs);
                            total_rhs.extend(rhs);

                            // If we ended up with zero total LHS and RHS, produce no event
                            if total_lhs.is_empty() && total_rhs.is_empty() {
                                return None;
                            }

                            // Otherwise, produce one single DoubleEntryRecordPayload event
                            let record = DoubleEntryRecordPayload {
                                height: berkeley_block.height,
                                state_hash: berkeley_block.state_hash.clone(),
                                ledger_destination: LedgerDestination::BlockchainLedger,
                                lhs: total_lhs,
                                rhs: total_rhs,
                            };

                            // Runtime check to verify LHS and RHS balance
                            record.verify();

                            let new_event = Event {
                                event_type: EventType::DoubleEntryTransaction,
                                payload: sonic_rs::to_string(&record).unwrap(),
                            };

                            // Return a single-event vector
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
            models::{FeeTransfer, FeeTransferViaCoinbase},
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
    fn create_double_entry_sink_node(id: &str) -> impl FnOnce() -> ActorNode {
        let sink_node_id = id.to_string();
        move || {
            ActorNodeBuilder::new(sink_node_id)
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
        // 1) Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // 2) Build the ActorDAG
        let mut dag = ActorDAG::new();

        // 3) Create the AccountingActor (root)
        let accounting_actor = AccountingActor::create_actor();
        let actor_id = accounting_actor.id();
        // set_root returns a Sender<Event>
        let actor_sender = dag.set_root(accounting_actor);

        // 4) Create the sink node, add to DAG, link from AccountingActor
        let sink_node_id = &"DoubleEntrySink".to_string();
        let sink_node = create_double_entry_sink_node(sink_node_id)();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, sink_node_id);

        // 5) Wrap + spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all(shutdown_rx).await;
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

        // 10) Shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_non_canonical_fee_transfer_with_coinbase() {
        // 1) Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // 2) Build the ActorDAG
        let mut dag = ActorDAG::new();

        // 3) Create the AccountingActor (root)
        let accounting_actor = AccountingActor::create_actor();
        let actor_id = accounting_actor.id();
        // set_root returns a Sender<Event>
        let actor_sender = dag.set_root(accounting_actor);

        // 4) Create the sink node, add to DAG, link from AccountingActor
        // Assume you already have a create_double_entry_sink_node(...) function
        let sink_node_id = &"DoubleEntrySink".to_string();
        let sink_node = create_double_entry_sink_node(sink_node_id)();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, sink_node_id);

        // 5) Wrap + spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all(shutdown_rx).await;
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

        // 10) Shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_canonical_fee_transfer_via_coinbase() {
        // 1) Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        // 2) Build the ActorDAG
        let mut dag = ActorDAG::new();

        // 3) Create the AccountingActor (root)
        let accounting_actor = AccountingActor::create_actor();
        let actor_id = accounting_actor.id();
        // set_root => returns a Sender<Event>
        let actor_sender = dag.set_root(accounting_actor);

        // 4) Create the sink node, add to DAG, link from AccountingActor
        let sink_node_id = &"DoubleEntrySink".to_string();
        let sink_node = create_double_entry_sink_node(sink_node_id)();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, sink_node_id);

        // 5) Wrap + spawn
        let dag = std::sync::Arc::new(tokio::sync::Mutex::new(dag));
        tokio::spawn({
            let dag = std::sync::Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all(shutdown_rx).await;
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
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        // 2) Build the ActorDAG
        let mut dag = ActorDAG::new();

        // 3) Create the AccountingActor (root)
        let accounting_actor = AccountingActor::create_actor();
        let actor_id = accounting_actor.id();
        let actor_sender = dag.set_root(accounting_actor);

        // 4) Create the sink node, add to DAG, link from AccountingActor
        let sink_node_id = &"DoubleEntrySink".to_string();
        let sink_node = create_double_entry_sink_node(sink_node_id)();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, sink_node_id);

        // 5) Wrap + spawn
        let dag = std::sync::Arc::new(tokio::sync::Mutex::new(dag));
        tokio::spawn({
            let dag = std::sync::Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all(shutdown_rx).await;
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

        // 10) Shutdown
        shutdown_tx.send(true).expect("Failed to send shutdown signal");
    }

    #[tokio::test]
    async fn test_canonical_coinbase_only() {
        // 1) Create the shutdown signal
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // 2) Build the ActorDAG
        let mut dag = ActorDAG::new();

        // 3) Create the AccountingActor (root)
        let accounting_actor = AccountingActor::create_actor();
        let actor_id = accounting_actor.id();
        let actor_sender = dag.set_root(accounting_actor);

        // 4) Create the sink node, add to DAG, link from AccountingActor
        let sink_node_id = &"DoubleEntrySink".to_string();
        let sink_node = create_double_entry_sink_node(sink_node_id)();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, sink_node_id);

        // 5) Wrap + spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all(shutdown_rx).await;
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
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // 2) Build the ActorDAG
        let mut dag = ActorDAG::new();

        // 3) Create the AccountingActor (root)
        let accounting_actor = AccountingActor::create_actor();
        let actor_id = accounting_actor.id();
        let actor_sender = dag.set_root(accounting_actor);

        // 4) Create the sink node, add to DAG, link from AccountingActor
        let sink_node_id = &"DoubleEntrySink".to_string();
        let sink_node = create_double_entry_sink_node(sink_node_id)();
        dag.add_node(sink_node);
        dag.link_parent(&actor_id, sink_node_id);

        // 5) Wrap + spawn
        let dag = Arc::new(Mutex::new(dag));
        tokio::spawn({
            let dag = Arc::clone(&dag);
            async move {
                dag.lock().await.spawn_all(shutdown_rx).await;
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
}
