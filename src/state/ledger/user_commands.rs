use mina_serialization_types::staged_ledger_diff::{TransactionStatus, SignedCommandPayloadBody, StakeDelegation, self};

use crate::block::precomputed::PrecomputedBlock;

use super::public_key::PublicKey;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash)]
pub enum UserCommandType {
    Payment,
    Delegation,
}

pub struct BalanceUpdate {
    pub public_key: PublicKey,
    pub balance: u64,
}

pub struct UserCommand {
    pub source_nonce: i32,
    pub command_type: UserCommandType,
    pub fee_payer: BalanceUpdate,
    pub source: BalanceUpdate,
    pub receiver: BalanceUpdate,
}

impl UserCommand {
    pub fn from_precomputed(precomputed_block: &PrecomputedBlock) -> Vec<Self> {
        precomputed_block.staged_ledger_diff.diff.t.0.t.t.commands
            .iter()
            .map(|command| command.t.clone())
            .flat_map(|command| {
                let source_nonce = match &command.data.t.t {
                    staged_ledger_diff::UserCommand::SignedCommand(signed_command) => {
                        signed_command.t.t.payload.t.t.common.t.t.t.nonce.t.t
                    },
                };
                if let TransactionStatus::Applied(_, balance_data_versioned) = command.status.t {
                    let mut delegation = false;
                    let (fee_payer, source, receiver) = match command.data.t.t {
                        staged_ledger_diff::UserCommand::SignedCommand(signed_command) => {
                            let fee_payer = signed_command.t.t.payload.t.t.common.t.t.t.fee_payer_pk;
                            match signed_command.t.t.payload.t.t.body.t.t {
                                SignedCommandPayloadBody::PaymentPayload(payment_payload) => {
                                    let payment_receiver = payment_payload.t.t.receiver_pk;
                                    let payment_source = payment_payload.t.t.source_pk;
                                    (fee_payer, payment_source, payment_receiver)
                                },
                                SignedCommandPayloadBody::StakeDelegation(delegation_payload) => {
                                    match delegation_payload.t {
                                        StakeDelegation::SetDelegate { delegator, new_delegate } => { 
                                            delegation = true;
                                            (fee_payer, delegator, new_delegate)
                                        },
                                    }
                                },
                            }
                        },
                    };

                    let balance_data_inner = balance_data_versioned.t;
                    let fee_payer_balance = balance_data_inner.fee_payer_balance
                        .map(|balance| balance.t.t.t);
                    let receiver_balance = balance_data_inner.receiver_balance
                        .map(|balance| balance.t.t.t);
                    let source_balance = balance_data_inner.source_balance
                        .map(|balance| balance.t.t.t);

                    if let (Some(fee_payer_balance), Some(receiver_balance), Some(source_balance)) 
                        = (fee_payer_balance, receiver_balance, source_balance) 
                    {
                        let user_command_type = if delegation { UserCommandType::Delegation } else { UserCommandType::Payment };
                        Some(UserCommand {
                            source_nonce,
                            command_type: user_command_type,
                            fee_payer: BalanceUpdate { public_key: fee_payer.into(), balance: fee_payer_balance },
                            source: BalanceUpdate { public_key: source.into(), balance: source_balance },
                            receiver: BalanceUpdate { public_key: receiver.into(), balance: receiver_balance }
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }
}