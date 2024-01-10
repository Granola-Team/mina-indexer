use crate::{
    block::precomputed::PrecomputedBlock,
    command::{signed::SignedCommand, Command},
    ledger::{Amount, PublicKey},
};
use mina_serialization_types::staged_ledger_diff::{SignedCommandPayloadCommon, UserCommand};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize)]
pub enum UpdateType {
    Deposit,
    Deduction,
}

#[derive(PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct PaymentDiff {
    pub public_key: PublicKey,
    pub amount: Amount,
    pub update_type: UpdateType,
}

#[derive(PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct DelegationDiff {
    pub delegator: PublicKey,
    pub delegate: PublicKey,
}

#[derive(PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub struct CoinbaseDiff {
    pub public_key: PublicKey,
    pub amount: Amount,
}

#[derive(PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub enum AccountDiff {
    Payment(PaymentDiff),
    Delegation(DelegationDiff),
    Coinbase(CoinbaseDiff),
}

impl AccountDiff {
    pub fn from_command(command: Command) -> Vec<Self> {
        match command {
            Command::Payment(payment) => vec![
                AccountDiff::Payment(PaymentDiff {
                    public_key: payment.source,
                    amount: payment.amount,
                    update_type: UpdateType::Deduction,
                }),
                AccountDiff::Payment(PaymentDiff {
                    public_key: payment.receiver,
                    amount: payment.amount,
                    update_type: UpdateType::Deposit,
                }),
            ],
            Command::Delegation(delegation) => {
                vec![AccountDiff::Delegation(DelegationDiff {
                    delegator: delegation.delegator,
                    delegate: delegation.delegate,
                })]
            }
        }
    }

    pub fn from_coinbase(coinbase_receiver: PublicKey, supercharge_coinbase: bool) -> Self {
        let amount = match supercharge_coinbase {
            true => 1440,
            false => 720,
        } * (1e9 as u64);
        AccountDiff::Coinbase(CoinbaseDiff {
            public_key: coinbase_receiver,
            amount: amount.into(),
        })
    }

    pub fn public_key(&self) -> PublicKey {
        match self {
            AccountDiff::Coinbase(coinbase_diff) => coinbase_diff.public_key.clone(),
            AccountDiff::Payment(payment_diff) => payment_diff.public_key.clone(),
            AccountDiff::Delegation(delegation_diff) => delegation_diff.delegator.clone(),
        }
    }

    pub fn from_block_fees(
        coinbase_receiver: PublicKey,
        precomputed_block: &PrecomputedBlock,
    ) -> Vec<AccountDiff> {
        precomputed_block
            .commands()
            .iter()
            .filter(|command| command.is_applied())
            .flat_map(
                |command| match command.0.clone().inner().data.inner().inner() {
                    UserCommand::SignedCommand(signed_command) => {
                        let SignedCommandPayloadCommon {
                            fee,
                            fee_token: _fee_token,
                            fee_payer_pk,
                            nonce: _nonce,
                            valid_until: _valid_until,
                            memo: _memo,
                        } = SignedCommand(signed_command).payload_common();
                        vec![
                            AccountDiff::Payment(PaymentDiff {
                                public_key: fee_payer_pk.into(),
                                amount: fee.clone().inner().inner().into(),
                                update_type: UpdateType::Deduction,
                            }),
                            AccountDiff::Payment(PaymentDiff {
                                public_key: coinbase_receiver.clone(),
                                amount: fee.inner().inner().into(),
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
            self.public_key, self.update_type, self.amount.0
        )
    }
}

impl std::fmt::Debug for DelegationDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} -> {:?}", self.delegate, self.delegator)
    }
}

impl std::fmt::Debug for CoinbaseDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} | Coinbase  | {}", self.public_key, self.amount.0)
    }
}

impl std::fmt::Debug for AccountDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountDiff::Coinbase(coin_diff) => write!(f, "Coinbase: {coin_diff:?}"),
            AccountDiff::Payment(pay_diff) => write!(f, "Payment:  {pay_diff:?}"),
            AccountDiff::Delegation(del_diff) => write!(f, "Delegation: {del_diff:?}"),
        }
    }
}

impl std::fmt::Debug for UpdateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateType::Deduction => write!(f, "Deduction"),
            UpdateType::Deposit => write!(f, "Deduction"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AccountDiff, CoinbaseDiff, DelegationDiff, PaymentDiff, UpdateType};
    use crate::{
        command::{Command, Delegation, Payment},
        ledger::{account::Amount, PublicKey},
    };

    // mainnet-220897-3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw for all tests below
    #[test]
    fn test_from_command() {
        let source_str = "B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG";
        let source_public_key_result = PublicKey::from_address(source_str).unwrap();
        let source_public_key = source_public_key_result.clone();
        let receiver_str = "B62qjoDXHMPZx8AACUrdaKVyDcn7uxbym1kxodgMXztn6iJC2yqEKbs";
        let receiver_public_key_result = PublicKey::from_address(receiver_str).unwrap();
        let receiver_public_key = receiver_public_key_result.clone();
        let payment_command = Command::Payment(Payment {
            source: source_public_key,
            receiver: receiver_public_key,
            amount: 536900000000.into(),
        });
        let expected_result = vec![
            AccountDiff::Payment(PaymentDiff {
                public_key: source_public_key_result,
                amount: 536900000000.into(),
                update_type: UpdateType::Deduction,
            }),
            AccountDiff::Payment(PaymentDiff {
                public_key: receiver_public_key_result,
                amount: 536900000000.into(),
                update_type: UpdateType::Deposit,
            }),
        ];

        assert_eq!(AccountDiff::from_command(payment_command), expected_result);
    }

    #[test]
    fn test_from_command_delegation() {
        let delegator_str = "B62qpYZ5BUaXq7gkUksirDA5c7okVMBY6VrQbj7YHLARWiBvu6A2fqi";
        let delegator_public_key_result = PublicKey::from_address(delegator_str).unwrap();
        let delegator_public_key = delegator_public_key_result.clone();
        let delegate_str = "B62qjSytpSK7aEauBprjXDSZwc9ai4YMv9tpmXLQK14Vy941YV36rMz";
        let delegate_public_key_result = PublicKey::from_address(delegate_str).unwrap();
        let delegate_public_key = delegate_public_key_result.clone();
        let delegation_command = Command::Delegation(Delegation {
            delegator: delegator_public_key,
            delegate: delegate_public_key,
        });
        let expected_result = vec![AccountDiff::Delegation(DelegationDiff {
            delegator: delegator_public_key_result,
            delegate: delegate_public_key_result,
        })];

        assert_eq!(
            AccountDiff::from_command(delegation_command),
            expected_result
        );
    }

    #[test]
    fn test_from_coinbase() {
        let coinbase_receiver_str = "B62qospDjUj43x2yMKiNehojWWRUsE1wpdUDVpfxH8V3n5Y1QgJKFfw";
        let coinbase_receiver_result = PublicKey::from_address(coinbase_receiver_str).unwrap();
        let coinbase_receiver = coinbase_receiver_result.clone();
        let supercharge_coinbase = true;
        let account_diff =
            AccountDiff::from_coinbase(coinbase_receiver_result, supercharge_coinbase);
        let expected_account_diff = AccountDiff::Coinbase(CoinbaseDiff {
            public_key: coinbase_receiver,
            amount: Amount(1440 * (1e9 as u64)),
        });

        assert_eq!(account_diff, expected_account_diff);
    }

    #[test]
    fn test_public_key_payment() {
        let payment_diff = PaymentDiff {
            public_key: PublicKey::from_address(
                "B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG",
            )
            .unwrap(),
            amount: 536900000000.into(),
            update_type: UpdateType::Deduction,
        };
        let account_diff = AccountDiff::Payment(payment_diff);
        let result = account_diff.public_key();
        let expected =
            PublicKey::from_address("B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG")
                .unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_public_key_delegation() {
        let delegation_diff = DelegationDiff {
            delegator: PublicKey::from_address(
                "B62qpYZ5BUaXq7gkUksirDA5c7okVMBY6VrQbj7YHLARWiBvu6A2fqi",
            )
            .unwrap(),
            delegate: PublicKey::from_address(
                "B62qjSytpSK7aEauBprjXDSZwc9ai4YMv9tpmXLQK14Vy941YV36rMz",
            )
            .unwrap(),
        };
        let account_diff = AccountDiff::Delegation(delegation_diff);
        let result = account_diff.public_key();
        let expected =
            PublicKey::from_address("B62qpYZ5BUaXq7gkUksirDA5c7okVMBY6VrQbj7YHLARWiBvu6A2fqi")
                .unwrap();
        assert_eq!(result, expected);
    }
}
