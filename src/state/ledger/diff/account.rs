use mina_serialization_types::{
    staged_ledger_diff::{SignedCommandPayloadCommon, UserCommand},
    v1::PublicKeyV1,
};
use serde::{Deserialize, Serialize};

use crate::{
    block::precomputed::PrecomputedBlock,
    state::ledger::{command::Command, PublicKey},
};

// add delegations later
#[derive(PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize)]
pub enum UpdateType {
    Deposit,
    Deduction,
}

#[derive(PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct PaymentDiff {
    pub public_key: PublicKey,
    pub amount: u64,
    pub update_type: UpdateType,
}

#[derive(PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct DelegationDiff {
    pub delegator: PublicKey,
    pub delegate: PublicKey,
}

#[derive(PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
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

    pub fn public_key(&self) -> PublicKey {
        match self {
            AccountDiff::Payment(payment_diff) => payment_diff.public_key.clone(),
            AccountDiff::Delegation(delegation_diff) => delegation_diff.delegator.clone(),
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

impl std::fmt::Debug for PaymentDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?} | {:?} | {}",
            self.public_key, self.update_type, self.amount
        )
    }
}

impl std::fmt::Debug for DelegationDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} -> {:?}", self.delegate, self.delegator)
    }
}

impl std::fmt::Debug for AccountDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountDiff::Payment(pay_diff) => write!(f, "Payment: {pay_diff:?}"),
            AccountDiff::Delegation(del_diff) => write!(f, "Delegation: {del_diff:?}"),
        }
    }
}

impl std::fmt::Debug for UpdateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateType::Deduction => write!(f, "Deduction"),
            UpdateType::Deposit => write!(f, "Deposit  "),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AccountDiff, PaymentDiff, UpdateType, DelegationDiff};
    use crate::state::ledger::{command::{Payment, Command, Delegation}};
    use crate::state::ledger::PublicKey;

    #[test]
    fn test_from_command() {
        // mainnet-220897-3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw

        let source_str = "B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG";
        let source_public_key_result = PublicKey::from_address(source_str).unwrap();
        let source_public_key = source_public_key_result.clone();

        let receiver_str = "B62qjoDXHMPZx8AACUrdaKVyDcn7uxbym1kxodgMXztn6iJC2yqEKbs";
        let receiver_public_key_result = PublicKey::from_address(receiver_str).unwrap();
        let receiver_public_key = receiver_public_key_result.clone();

        let payment_command = Command::Payment(Payment {
            source: source_public_key.into(),
            receiver: receiver_public_key.into(),
            amount: 536900000000,
        });

        let expected_result = vec![
            AccountDiff::Payment(PaymentDiff {
                public_key: source_public_key_result.into(),
                amount: 536900000000,
                update_type: UpdateType::Deduction,
            }),
            AccountDiff::Payment(PaymentDiff {
                public_key: receiver_public_key_result.into(),
                amount: 536900000000,
                update_type: UpdateType::Deposit,
            }),
        ];

        assert_eq!(AccountDiff::from_command(payment_command), expected_result);

    }

    #[test]
    fn test_from_command_delegation() {
        // mainnet-220897-3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw

        let delegator_str = "B62qpYZ5BUaXq7gkUksirDA5c7okVMBY6VrQbj7YHLARWiBvu6A2fqi";
        let delegator_public_key_result = PublicKey::from_address(delegator_str).unwrap();
        let delegator_public_key = delegator_public_key_result.clone();

        let delegate_str = "B62qjSytpSK7aEauBprjXDSZwc9ai4YMv9tpmXLQK14Vy941YV36rMz";
        let delegate_public_key_result = PublicKey::from_address(delegate_str).unwrap();
        let delegate_public_key = delegate_public_key_result.clone();

        let delegation_command = Command::Delegation(Delegation {
            delegator: delegator_public_key.into(),
            delegate: delegate_public_key.into(),
        });

        let expected_result = vec![AccountDiff::Delegation(DelegationDiff {
            delegator: delegator_public_key_result.into(),
            delegate: delegate_public_key_result.into(),
        })];

        assert_eq!(AccountDiff::from_command(delegation_command), expected_result);
    }

}
