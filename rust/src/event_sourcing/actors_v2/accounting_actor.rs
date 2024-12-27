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
        if canonical {
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
        if canonical {
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
