use mina_serialization_types::{
    staged_ledger_diff::{SignedCommandPayloadCommon, UserCommand},
    v1::PublicKeyV1,
};

use crate::{
    precomputed_block::PrecomputedBlock,
    state::ledger::{command::Command, PublicKey},
};

// add delegations later
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum UpdateType {
    Deposit,
    Deduction,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct PaymentDiff {
    pub public_key: PublicKey,
    pub amount: u64,
    pub update_type: UpdateType,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct DelegationDiff {
    pub delegator: PublicKey,
    pub delegate: PublicKey,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum AccountDiff {
    Payment(PaymentDiff),
    Delegation(DelegationDiff),
}

impl AccountDiff {
    pub fn from_command(command: Command) -> Vec<Self> {
        match command {
            Command::Payment(payment) => vec![
                AccountDiff::Payment(PaymentDiff {
                    public_key: payment.source.into(),
                    amount: payment.amount,
                    update_type: UpdateType::Deduction,
                }),
                AccountDiff::Payment(PaymentDiff {
                    public_key: payment.receiver.into(),
                    amount: payment.amount,
                    update_type: UpdateType::Deposit,
                }),
            ],
            Command::Delegation(delegation) => vec![AccountDiff::Delegation(DelegationDiff {
                delegator: delegation.delegator.into(),
                delegate: delegation.delegate.into(),
            })],
        }
    }

    pub fn from_coinbase(coinbase_receiver: PublicKeyV1, supercharge_coinbase: bool) -> Self {
        let amount = match supercharge_coinbase {
            true => 1440,
            false => 720,
        } * (1e9 as u64);
        AccountDiff::Payment(PaymentDiff {
            public_key: coinbase_receiver.into(),
            amount,
            update_type: UpdateType::Deposit,
        })
    }

    pub fn public_key(&self) -> PublicKeyV1 {
        match self {
            AccountDiff::Payment(payment_diff) => payment_diff.public_key.clone().into(),
            AccountDiff::Delegation(delegation_diff) => delegation_diff.delegator.clone().into(),
        }
    }

    pub fn from_block_fees(
        coinbase_receiver: PublicKeyV1,
        precomputed_block: &PrecomputedBlock,
    ) -> Vec<AccountDiff> {
        precomputed_block
            .staged_ledger_diff
            .clone()
            .diff
            .inner()
            .0
            .inner()
            .inner()
            .commands
            .iter()
            .flat_map(
                |command| match command.clone().inner().data.inner().inner() {
                    UserCommand::SignedCommand(signed_command) => {
                        let SignedCommandPayloadCommon {
                            fee,
                            fee_token: _fee_token,
                            fee_payer_pk,
                            nonce: _nonce,
                            valid_until: _valid_until,
                            memo: _memo,
                        } = signed_command
                            .inner()
                            .inner()
                            .payload
                            .inner()
                            .inner()
                            .common
                            .inner()
                            .inner()
                            .inner();
                        vec![
                            AccountDiff::Payment(PaymentDiff {
                                public_key: fee_payer_pk.into(),
                                amount: fee.clone().inner().inner(),
                                update_type: UpdateType::Deduction,
                            }),
                            AccountDiff::Payment(PaymentDiff {
                                public_key: coinbase_receiver.clone().into(),
                                amount: fee.inner().inner(),
                                update_type: UpdateType::Deposit,
                            }),
                        ]
                    }
                },
            )
            .collect()
    }
}
