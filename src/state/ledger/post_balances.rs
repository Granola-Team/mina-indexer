use crate::block::{precomputed::PrecomputedBlock, signed_command::SignedCommand};

use super::{
    command::{CommandStatusData, UserCommandWithStatus},
    public_key::PublicKey,
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash)]
pub enum UserCommandType {
    Payment,
    Delegation,
}

pub struct PostBalance {
    pub public_key: PublicKey,
    pub balance: u64,
}

pub struct PostBalanceUpdate {
    pub source_nonce: i32,
    pub command_type: UserCommandType,
    pub fee_payer: PostBalance,
    pub source: PostBalance,
    pub receiver: PostBalance,
}

impl PostBalanceUpdate {
    pub fn from_precomputed(precomputed_block: &PrecomputedBlock) -> Vec<Self> {
        precomputed_block
            .commands()
            .iter()
            .map(|command| UserCommandWithStatus(command.clone()))
            .flat_map(|command| {
                let signed_command = SignedCommand::from_user_command(command.clone());
                let source_nonce = signed_command.source_nonce();
                if let CommandStatusData::Applied { balance_data } = command.status_data() {
                    let delegation = signed_command.is_delegation();
                    let fee_payer = signed_command.fee_payer();
                    let source = signed_command.source_pk();
                    let receiver = signed_command.receiver_pk();

                    let fee_payer_balance =
                        balance_data.fee_payer_balance.map(|balance| balance.t.t.t);
                    let receiver_balance =
                        balance_data.receiver_balance.map(|balance| balance.t.t.t);
                    let source_balance = balance_data.source_balance.map(|balance| balance.t.t.t);

                    if let (Some(fee_payer_balance), Some(receiver_balance), Some(source_balance)) =
                        (fee_payer_balance, receiver_balance, source_balance)
                    {
                        let user_command_type = if delegation {
                            UserCommandType::Delegation
                        } else {
                            UserCommandType::Payment
                        };
                        Some(PostBalanceUpdate {
                            source_nonce,
                            command_type: user_command_type,
                            fee_payer: PostBalance {
                                public_key: fee_payer,
                                balance: fee_payer_balance,
                            },
                            source: PostBalance {
                                public_key: source,
                                balance: source_balance,
                            },
                            receiver: PostBalance {
                                public_key: receiver,
                                balance: receiver_balance,
                            },
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
