use crate::{
    block::precomputed::PrecomputedBlock,
    command::{signed::SignedCommand, CommandStatusData, CommandType, CommandUpdate},
    ledger::{coinbase::Coinbase, public_key::PublicKey},
};

pub enum PostBalanceUpdate {
    User(CommandUpdate),
    Coinbase(PostBalance),
    FeeTransfer(FeeTransferUpdate),
}

pub enum FeeTransferUpdate {
    One(PostBalance),
    Two(PostBalance, PostBalance),
}

pub struct PostBalance {
    pub public_key: PublicKey,
    pub balance: u64,
}

impl PostBalanceUpdate {
    /// Compute a post balance update from the givien block
    pub fn from_precomputed(precomputed_block: &PrecomputedBlock) -> Vec<Self> {
        // user commands updates
        let mut commands: Vec<Self> = precomputed_block
            .commands()
            .iter()
            .flat_map(|command| {
                let status = command.status_data();
                let signed_command = SignedCommand::from_user_command(command.clone());
                let source_nonce = signed_command.source_nonce();
                let fee_payer = signed_command.fee_payer_pk();
                let source = signed_command.source_pk();
                let receiver = signed_command.receiver_pk();
                let fee_payer_balance = CommandStatusData::fee_payer_balance(&status);
                let receiver_balance = CommandStatusData::receiver_balance(&status);
                let source_balance = CommandStatusData::source_balance(&status);

                if let (Some(fee_payer_balance), Some(receiver_balance), Some(source_balance)) =
                    (fee_payer_balance, receiver_balance, source_balance)
                {
                    let user_command_type = if signed_command.is_delegation() {
                        CommandType::Delegation
                    } else {
                        CommandType::Payment
                    };

                    return Some(PostBalanceUpdate::User(CommandUpdate {
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
                    }));
                }

                None
            })
            .collect();

        // coinbase update
        if let Some(balance) = precomputed_block
            .coinbase_receiver_balance()
            .filter(|_| Coinbase::from_precomputed(precomputed_block).is_coinbase_applied())
        {
            commands.push(PostBalanceUpdate::Coinbase(PostBalance {
                public_key: precomputed_block.coinbase_receiver(),
                balance,
            }));
        }

        commands
    }
}
