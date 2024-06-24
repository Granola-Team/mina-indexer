use crate::{
    block::precomputed::PrecomputedBlock,
    command::{signed::SignedCommand, Command, UserCommandWithStatus},
    ledger::{coinbase::Coinbase, Amount, PublicKey},
    snark_work::SnarkWorkSummary,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Serialize, Deserialize)]
pub enum UpdateType {
    /// Carries Some(nonce) for a user command, None for internal command
    Debit(Option<u32>),
    Credit,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct PaymentDiff {
    pub update_type: UpdateType,
    pub public_key: PublicKey,
    pub amount: Amount,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct DelegationDiff {
    pub nonce: u32,
    pub delegator: PublicKey,
    pub delegate: PublicKey,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct CoinbaseDiff {
    pub public_key: PublicKey,
    pub amount: Amount,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct FailedTransactionNonceDiff {
    pub public_key: PublicKey,
    pub nonce: u32,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub enum AccountDiff {
    Payment(PaymentDiff),
    Delegation(DelegationDiff),
    Coinbase(CoinbaseDiff),
    FeeTransfer(PaymentDiff),
    /// Overrides the fee transfer for SNARK work
    FeeTransferViaCoinbase(PaymentDiff),
    /// Updates the nonce for a failed txn
    FailedTransactionNonce(FailedTransactionNonceDiff),
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum AccountDiffType {
    Payment(u32),
    Delegation(u32),
    Coinbase,
    FeeTransfer,
    FeeTransferViaCoinbase,
}

impl AccountDiff {
    pub fn from_command(command: Command) -> Vec<Self> {
        match command {
            Command::Payment(payment) => vec![
                AccountDiff::Payment(PaymentDiff {
                    public_key: payment.source,
                    amount: payment.amount,
                    update_type: UpdateType::Debit(Some(payment.nonce)),
                }),
                AccountDiff::Payment(PaymentDiff {
                    public_key: payment.receiver,
                    amount: payment.amount,
                    update_type: UpdateType::Credit,
                }),
            ],
            Command::Delegation(delegation) => {
                vec![AccountDiff::Delegation(DelegationDiff {
                    delegator: delegation.delegator,
                    delegate: delegation.delegate,
                    nonce: delegation.nonce,
                })]
            }
        }
    }

    pub fn from_coinbase(coinbase: Coinbase) -> Vec<Self> {
        let mut res = vec![Self::Coinbase(CoinbaseDiff {
            public_key: coinbase.receiver.clone(),
            amount: coinbase.amount().into(),
        })];

        res.append(
            &mut coinbase
                .fee_transfer()
                .into_iter()
                .map(Self::FeeTransferViaCoinbase)
                .collect(),
        );
        res
    }

    pub fn public_key(&self) -> PublicKey {
        match self {
            Self::Payment(payment_diff) => payment_diff.public_key.clone(),
            Self::Delegation(delegation_diff) => delegation_diff.delegator.clone(),
            Self::Coinbase(coinbase_diff) => coinbase_diff.public_key.clone(),
            Self::FeeTransfer(fee_transfer_diff) => fee_transfer_diff.public_key.clone(),
            Self::FeeTransferViaCoinbase(fee_transfer_diff) => fee_transfer_diff.public_key.clone(),
            Self::FailedTransactionNonce(failed_diff) => failed_diff.public_key.clone(),
        }
    }

    fn transaction_fees(
        coinbase_receiver: &PublicKey,
        user_cmds: Vec<UserCommandWithStatus>,
    ) -> Vec<Self> {
        let mut fee_map = HashMap::new();
        user_cmds.iter().for_each(|user_cmd| {
            let signed_cmd = SignedCommand::from_user_command(user_cmd.clone());
            let fee_payer = signed_cmd.fee_payer_pk();
            let fee = signed_cmd.fee();
            match fee_map.get_mut(&fee_payer) {
                None => {
                    fee_map.insert(fee_payer.clone(), fee);
                }
                Some(acc) => *acc += fee,
            }
        });
        fee_map
            .iter()
            .flat_map(|(pk, fee)| {
                let mut res = vec![];
                if *fee > 0 {
                    res.push(Self::FeeTransfer(PaymentDiff {
                        public_key: pk.clone(),
                        amount: (*fee).into(),
                        update_type: UpdateType::Debit(None),
                    }));
                    res.push(Self::FeeTransfer(PaymentDiff {
                        public_key: coinbase_receiver.clone(),
                        amount: (*fee).into(),
                        update_type: UpdateType::Credit,
                    }));
                }
                res
            })
            .collect()
    }

    /// Fees for user commands, applied or failed, aggregated per public key
    pub fn from_transaction_fees(precomputed_block: &PrecomputedBlock) -> Vec<Self> {
        let coinbase_receiver = &precomputed_block.coinbase_receiver();
        let mut fees =
            Self::transaction_fees(coinbase_receiver, precomputed_block.commands_pre_diff());
        fees.append(&mut Self::transaction_fees(
            coinbase_receiver,
            precomputed_block.commands_post_diff(),
        ));
        fees
    }

    /// Fees for SNARK work, aggregated per public key
    pub fn from_snark_fees(precomputed_block: &PrecomputedBlock) -> Vec<Self> {
        let snark_fees = SnarkWorkSummary::from_precomputed(precomputed_block);
        let mut fee_map = HashMap::new();
        snark_fees
            .iter()
            .for_each(|snark| match fee_map.get_mut(&snark.prover) {
                None => {
                    fee_map.insert(snark.prover.clone(), snark.fee);
                }
                Some(cum_fee) => *cum_fee += snark.fee,
            });
        fee_map
            .iter()
            .flat_map(|(prover, total_fee)| {
                let mut res = vec![];
                if *total_fee > 0 {
                    res.push(AccountDiff::FeeTransfer(PaymentDiff {
                        public_key: precomputed_block.coinbase_receiver(),
                        amount: (*total_fee).into(),
                        update_type: UpdateType::Debit(None),
                    }));
                    res.push(AccountDiff::FeeTransfer(PaymentDiff {
                        public_key: prover.clone(),
                        amount: (*total_fee).into(),
                        update_type: UpdateType::Credit,
                    }));
                }
                res
            })
            .collect()
    }

    /// User command + SNARK work fees, aggregated per public key
    pub fn from_block_fees(precomputed_block: &PrecomputedBlock) -> Vec<Self> {
        let mut fees = Self::from_transaction_fees(precomputed_block);
        fees.append(&mut Self::from_snark_fees(precomputed_block));
        fees
    }

    pub fn from(
        sender: &str,
        receiver: &str,
        diff_type: AccountDiffType,
        amount: u64,
    ) -> Vec<Self> {
        match diff_type {
            AccountDiffType::Payment(nonce) => vec![
                Self::Payment(PaymentDiff {
                    public_key: sender.into(),
                    amount: amount.into(),
                    update_type: UpdateType::Debit(Some(nonce)),
                }),
                Self::Payment(PaymentDiff {
                    public_key: receiver.into(),
                    amount: amount.into(),
                    update_type: UpdateType::Credit,
                }),
            ],
            AccountDiffType::Delegation(nonce) => vec![Self::Delegation(DelegationDiff {
                delegate: sender.into(),
                delegator: receiver.into(),
                nonce,
            })],
            AccountDiffType::Coinbase => vec![Self::Coinbase(CoinbaseDiff {
                public_key: sender.into(),
                amount: amount.into(),
            })],
            AccountDiffType::FeeTransfer => vec![
                Self::FeeTransfer(PaymentDiff {
                    public_key: sender.into(),
                    amount: amount.into(),
                    update_type: UpdateType::Debit(None),
                }),
                Self::FeeTransfer(PaymentDiff {
                    public_key: receiver.into(),
                    amount: amount.into(),
                    update_type: UpdateType::Credit,
                }),
            ],
            AccountDiffType::FeeTransferViaCoinbase => vec![
                Self::FeeTransferViaCoinbase(PaymentDiff {
                    public_key: sender.into(),
                    amount: amount.into(),
                    update_type: UpdateType::Debit(None),
                }),
                Self::FeeTransferViaCoinbase(PaymentDiff {
                    public_key: receiver.into(),
                    amount: amount.into(),
                    update_type: UpdateType::Credit,
                }),
            ],
        }
    }
}

impl PaymentDiff {
    pub fn from_account_diff(diff: AccountDiff) -> Option<Self> {
        match diff {
            AccountDiff::Payment(diff)
            | AccountDiff::FeeTransfer(diff)
            | AccountDiff::FeeTransferViaCoinbase(diff) => Some(diff),
            AccountDiff::Coinbase(cb_diff) => Some(Self {
                update_type: UpdateType::Credit,
                public_key: cb_diff.public_key,
                amount: cb_diff.amount,
            }),
            AccountDiff::Delegation(_) | AccountDiff::FailedTransactionNonce(_) => None,
        }
    }
}

impl std::fmt::Debug for PaymentDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} | {:?} | {}",
            self.public_key, self.update_type, self.amount.0
        )
    }
}

impl std::fmt::Debug for DelegationDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} -({})-> {}",
            self.delegate, self.nonce, self.delegator
        )
    }
}

impl std::fmt::Debug for CoinbaseDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} | Credit | {}", self.public_key, self.amount.0)
    }
}

impl std::fmt::Debug for FailedTransactionNonceDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} | Nonce {}", self.public_key, self.nonce)
    }
}

impl std::fmt::Debug for AccountDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountDiff::Payment(pay_diff) => write!(f, "Payment:      {pay_diff:?}"),
            AccountDiff::Delegation(del_diff) => write!(f, "Delegation:   {del_diff:?}"),
            AccountDiff::Coinbase(coin_diff) => write!(f, "Coinbase:     {coin_diff:?}"),
            AccountDiff::FeeTransfer(pay_diff) => write!(f, "Fee transfer: {pay_diff:?}"),
            AccountDiff::FeeTransferViaCoinbase(pay_diff) => {
                write!(f, "Fee transfer via coinbase: {pay_diff:?}")
            }
            AccountDiff::FailedTransactionNonce(failed_diff) => {
                write!(f, "Failed transaction: {failed_diff:?}")
            }
        }
    }
}

impl std::fmt::Debug for UpdateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateType::Debit(nonce) => write!(
                f,
                "Debit {}",
                nonce.map(|n| n.to_string()).unwrap_or_default()
            ),
            UpdateType::Credit => write!(f, "Credit"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AccountDiff, CoinbaseDiff, DelegationDiff, PaymentDiff, UpdateType};
    use crate::{
        command::{Command, Delegation, Payment},
        ledger::{
            account::Amount,
            coinbase::{Coinbase, CoinbaseFeeTransfer, CoinbaseKind},
            PublicKey,
        },
    };

    #[test]
    fn test_fee_transfer_via_coinbase() {
        let fee = 10000000;
        let receiver: PublicKey = "B62qkMUJyt7LmPnfu8in6qshaQSvTgLgNjx6h7YySRJ28wJegJ82n6u".into();
        let snarker: PublicKey = "B62qospDjUj43x2yMKiNehojWWRUsE1wpdUDVpfxH8V3n5Y1QgJKFfw".into();
        let account_diff = AccountDiff::from_coinbase(Coinbase {
            supercharge: true,
            is_new_account: true,
            receiver: receiver.clone(),
            receiver_balance: Some(1439 * (1e9 as u64)),
            kind: CoinbaseKind::One(Some(CoinbaseFeeTransfer {
                receiver_pk: snarker.clone(),
                fee,
            })),
        });
        let expected_account_diff = vec![
            AccountDiff::Coinbase(CoinbaseDiff {
                public_key: receiver.clone(),
                amount: Amount(1439 * (1e9 as u64)),
            }),
            AccountDiff::FeeTransferViaCoinbase(PaymentDiff {
                public_key: receiver,
                amount: fee.into(),
                update_type: UpdateType::Debit(None),
            }),
            AccountDiff::FeeTransferViaCoinbase(PaymentDiff {
                public_key: snarker,
                amount: fee.into(),
                update_type: UpdateType::Credit,
            }),
        ];

        assert_eq!(account_diff, expected_account_diff);
    }

    // mainnet-220897-3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw for all
    // tests below
    #[test]
    fn test_from_command() {
        let source_str = "B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG";
        let source_public_key = PublicKey::new(source_str);
        let receiver_str = "B62qjoDXHMPZx8AACUrdaKVyDcn7uxbym1kxodgMXztn6iJC2yqEKbs";
        let receiver_public_key = PublicKey::new(receiver_str);
        let nonce = 5;

        let payment_command = Command::Payment(Payment {
            source: source_public_key.clone(),
            receiver: receiver_public_key.clone(),
            amount: 536900000000.into(),
            nonce,
        });
        let expected_result = vec![
            AccountDiff::Payment(PaymentDiff {
                public_key: source_public_key,
                amount: 536900000000.into(),
                update_type: UpdateType::Debit(Some(nonce)),
            }),
            AccountDiff::Payment(PaymentDiff {
                public_key: receiver_public_key,
                amount: 536900000000.into(),
                update_type: UpdateType::Credit,
            }),
        ];

        assert_eq!(AccountDiff::from_command(payment_command), expected_result);
    }

    #[test]
    fn test_from_command_delegation() {
        let delegator_str = "B62qpYZ5BUaXq7gkUksirDA5c7okVMBY6VrQbj7YHLARWiBvu6A2fqi";
        let delegator_public_key = PublicKey::new(delegator_str);
        let delegate_str = "B62qjSytpSK7aEauBprjXDSZwc9ai4YMv9tpmXLQK14Vy941YV36rMz";
        let delegate_public_key = PublicKey::new(delegate_str);
        let nonce = 42;

        let delegation_command = Command::Delegation(Delegation {
            delegator: delegator_public_key.clone(),
            delegate: delegate_public_key.clone(),
            nonce,
        });
        let expected_result = vec![AccountDiff::Delegation(DelegationDiff {
            delegator: delegator_public_key,
            delegate: delegate_public_key,
            nonce,
        })];

        assert_eq!(
            AccountDiff::from_command(delegation_command),
            expected_result
        );
    }

    #[test]
    fn test_from_coinbase() {
        let receiver: PublicKey = "B62qospDjUj43x2yMKiNehojWWRUsE1wpdUDVpfxH8V3n5Y1QgJKFfw".into();
        let account_diff = AccountDiff::from_coinbase(Coinbase {
            supercharge: true,
            is_new_account: false,
            receiver_balance: None,
            receiver: receiver.clone(),
            kind: CoinbaseKind::One(None),
        });
        let expected_account_diff = vec![AccountDiff::Coinbase(CoinbaseDiff {
            public_key: receiver,
            amount: Amount(1440 * (1e9 as u64)),
        })];

        assert_eq!(account_diff, expected_account_diff);
    }

    #[test]
    fn test_public_key_payment() {
        let nonce = 42;
        let payment_diff = PaymentDiff {
            public_key: PublicKey::new("B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG"),
            amount: 536900000000.into(),
            update_type: UpdateType::Debit(Some(nonce)),
        };
        let account_diff = AccountDiff::Payment(payment_diff);
        let result = account_diff.public_key();
        let expected = PublicKey::new("B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG");

        assert_eq!(result, expected);
    }

    #[test]
    fn test_public_key_delegation() {
        let nonce = 42;
        let delegation_diff = DelegationDiff {
            delegator: PublicKey::new("B62qpYZ5BUaXq7gkUksirDA5c7okVMBY6VrQbj7YHLARWiBvu6A2fqi"),
            delegate: PublicKey::new("B62qjSytpSK7aEauBprjXDSZwc9ai4YMv9tpmXLQK14Vy941YV36rMz"),
            nonce,
        };
        let account_diff = AccountDiff::Delegation(delegation_diff);
        let result = account_diff.public_key();
        let expected = PublicKey::new("B62qpYZ5BUaXq7gkUksirDA5c7okVMBY6VrQbj7YHLARWiBvu6A2fqi");
        assert_eq!(result, expected);
    }
}
