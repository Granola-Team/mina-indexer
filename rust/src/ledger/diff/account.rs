use crate::{
    block::precomputed::PrecomputedBlock,
    command::{signed::SignedCommand, Command, UserCommandWithStatus},
    ledger::{account::Nonce, coinbase::Coinbase, signed_amount::SignedAmount, Amount, PublicKey},
    snark_work::SnarkWorkSummary,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Serialize, Deserialize)]
pub enum UpdateType {
    /// Carries Some(nonce) for a user command, None for internal command
    Debit(Option<Nonce>),
    Credit,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct PaymentDiff {
    pub update_type: UpdateType,
    pub public_key: PublicKey,
    pub amount: Amount, // deprecated
    pub signed_amount: Option<SignedAmount>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct DelegationDiff {
    pub nonce: Nonce,
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
    pub nonce: Nonce,
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

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub enum UnapplyAccountDiff {
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
    Payment(Nonce),
    Delegation(Nonce),
    Coinbase,
    FeeTransfer,
    FeeTransferViaCoinbase,
}

impl AccountDiff {
    pub fn from_command(command: Command) -> Vec<Vec<Self>> {
        match command {
            Command::Payment(payment) => {
                vec![vec![
                    Self::Payment(PaymentDiff {
                        public_key: payment.receiver,
                        amount: payment.amount,
                        signed_amount: None,
                        update_type: UpdateType::Credit,
                    }),
                    Self::Payment(PaymentDiff {
                        public_key: payment.source,
                        amount: payment.amount,
                        signed_amount: None,
                        update_type: UpdateType::Debit(Some(payment.nonce + 1)),
                    }),
                ]]
            }
            Command::Delegation(delegation) => {
                vec![vec![AccountDiff::Delegation(DelegationDiff {
                    delegator: delegation.delegator,
                    delegate: delegation.delegate,
                    nonce: delegation.nonce + 1,
                })]]
            }
        }
    }

    pub fn unapply(self) -> UnapplyAccountDiff {
        match self {
            Self::Coinbase(diff) => UnapplyAccountDiff::Coinbase(diff),
            Self::Payment(diff) => UnapplyAccountDiff::Payment(diff),
            Self::FeeTransfer(diff) => UnapplyAccountDiff::FeeTransfer(diff),
            Self::FeeTransferViaCoinbase(diff) => UnapplyAccountDiff::FeeTransferViaCoinbase(diff),
            Self::Delegation(diff) => UnapplyAccountDiff::Delegation(diff),
            Self::FailedTransactionNonce(diff) => UnapplyAccountDiff::FailedTransactionNonce(diff),
        }
    }

    pub fn from_coinbase(coinbase: Coinbase) -> Vec<Vec<Self>> {
        let mut res = vec![vec![Self::Coinbase(CoinbaseDiff {
            public_key: coinbase.receiver.clone(),
            amount: coinbase.amount().into(),
        })]];
        res.append(
            &mut coinbase
                .fee_transfer()
                .into_iter()
                .map(|pair| pair.into_iter().map(Self::FeeTransferViaCoinbase).collect())
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
    ) -> Vec<Vec<Self>> {
        let mut fee_map = HashMap::new();
        for user_cmd in user_cmds.iter() {
            let signed_cmd = SignedCommand::from_user_command(user_cmd.clone());
            let fee_payer = signed_cmd.fee_payer_pk();
            let fee = signed_cmd.fee();
            fee_map
                .entry(fee_payer)
                .and_modify(|acc| *acc += fee)
                .or_insert(fee);
        }
        fee_map
            .iter()
            .flat_map(|(pk, fee)| {
                let mut res = vec![];
                if *fee > 0 {
                    res.push(vec![
                        Self::FeeTransfer(PaymentDiff {
                            public_key: coinbase_receiver.clone(),
                            amount: (*fee).into(),
                            signed_amount: None,
                            update_type: UpdateType::Credit,
                        }),
                        Self::FeeTransfer(PaymentDiff {
                            public_key: pk.clone(),
                            amount: (*fee).into(),
                            signed_amount: None,
                            update_type: UpdateType::Debit(None),
                        }),
                    ]);
                }
                res
            })
            .collect()
    }

    /// Fees for user commands, applied or failed, aggregated per public key
    fn from_transaction_fees(precomputed_block: &PrecomputedBlock) -> Vec<Vec<Self>> {
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
    pub fn from_snark_fees(precomputed_block: &PrecomputedBlock) -> Vec<Vec<Self>> {
        let snarks = SnarkWorkSummary::from_precomputed(precomputed_block);
        let mut fee_map = HashMap::new();
        // SNARK work fees aggregated per public key
        for snark in snarks {
            fee_map
                .entry(snark.prover.clone())
                .and_modify(|agg_fee| *agg_fee += snark.fee)
                .or_insert(snark.fee);
        }

        fee_map
            .iter()
            .flat_map(|(prover, total_fee)| {
                let mut res = vec![];
                // No need to issue Debits and Credits if the fee is 0
                if *total_fee > 0 {
                    res.push(vec![
                        AccountDiff::FeeTransfer(PaymentDiff {
                            public_key: prover.clone(),
                            amount: (*total_fee).into(),
                            signed_amount: None,
                            update_type: UpdateType::Credit,
                        }),
                        AccountDiff::FeeTransfer(PaymentDiff {
                            public_key: precomputed_block.coinbase_receiver(),
                            amount: (*total_fee).into(),
                            signed_amount: None,
                            update_type: UpdateType::Debit(None),
                        }),
                    ]);
                }
                res
            })
            .collect::<Vec<_>>()
    }

    /// User command + SNARK work fees, aggregated per public key
    pub fn from_block_fees(precomputed_block: &PrecomputedBlock) -> Vec<Vec<Self>> {
        let mut fees = Self::from_transaction_fees(precomputed_block);
        fees.append(&mut Self::from_snark_fees(precomputed_block));
        fees
    }

    pub fn amount(&self) -> i64 {
        match self {
            Self::Delegation(_) | Self::FailedTransactionNonce(_) => 0,
            Self::Coinbase(diff) => diff.amount.0 as i64,
            Self::FeeTransfer(diff) | Self::FeeTransferViaCoinbase(diff) | Self::Payment(diff) => {
                match diff.update_type {
                    UpdateType::Credit => diff.amount.0 as i64,
                    UpdateType::Debit(_) => 0 - diff.amount.0 as i64,
                }
            }
        }
    }

    pub fn from(
        sender: &str,
        receiver: &str,
        diff_type: AccountDiffType,
        amount: u64,
    ) -> Vec<Vec<Self>> {
        match diff_type {
            AccountDiffType::Payment(nonce) => vec![vec![
                Self::Payment(PaymentDiff {
                    public_key: receiver.into(),
                    amount: amount.into(),
                    signed_amount: None,
                    update_type: UpdateType::Credit,
                }),
                Self::Payment(PaymentDiff {
                    public_key: sender.into(),
                    amount: amount.into(),
                    signed_amount: None,
                    update_type: UpdateType::Debit(Some(nonce)),
                }),
            ]],
            AccountDiffType::Delegation(nonce) => vec![vec![Self::Delegation(DelegationDiff {
                delegate: sender.into(),
                delegator: receiver.into(),
                nonce,
            })]],
            AccountDiffType::Coinbase => vec![vec![Self::Coinbase(CoinbaseDiff {
                public_key: sender.into(),
                amount: amount.into(),
            })]],
            AccountDiffType::FeeTransfer => vec![vec![
                Self::FeeTransfer(PaymentDiff {
                    public_key: receiver.into(),
                    amount: amount.into(),
                    signed_amount: None,
                    update_type: UpdateType::Credit,
                }),
                Self::FeeTransfer(PaymentDiff {
                    public_key: sender.into(),
                    amount: amount.into(),
                    signed_amount: None,
                    update_type: UpdateType::Debit(None),
                }),
            ]],
            AccountDiffType::FeeTransferViaCoinbase => vec![vec![
                Self::FeeTransferViaCoinbase(PaymentDiff {
                    public_key: receiver.into(),
                    amount: amount.into(),
                    signed_amount: None,
                    update_type: UpdateType::Credit,
                }),
                Self::FeeTransferViaCoinbase(PaymentDiff {
                    public_key: sender.into(),
                    amount: amount.into(),
                    signed_amount: None,
                    update_type: UpdateType::Debit(None),
                }),
            ]],
        }
    }
}

impl PaymentDiff {
    pub fn from_account_diff(diff: AccountDiff) -> Option<Self> {
        use AccountDiff::*;
        match diff {
            Payment(diff) | FeeTransfer(diff) | FeeTransferViaCoinbase(diff) => Some(diff),
            Coinbase(cb_diff) => Some(Self {
                update_type: UpdateType::Credit,
                public_key: cb_diff.public_key,
                amount: cb_diff.amount,
                signed_amount: None,
            }),
            Delegation(_) | FailedTransactionNonce(_) => None,
        }
    }

    pub fn unapply(self) -> Self {
        if self.update_type == UpdateType::Credit {
            Self {
                update_type: UpdateType::Debit(None),
                ..self
            }
        } else {
            Self {
                update_type: UpdateType::Credit,
                ..self
            }
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
        use AccountDiff::*;
        match self {
            Payment(pay_diff) => write!(f, "Payment:      {pay_diff:?}"),
            Delegation(del_diff) => write!(f, "Delegation:   {del_diff:?}"),
            Coinbase(coin_diff) => write!(f, "Coinbase:     {coin_diff:?}"),
            FeeTransfer(pay_diff) => write!(f, "Fee transfer: {pay_diff:?}"),
            FeeTransferViaCoinbase(pay_diff) => {
                write!(f, "Fee transfer via coinbase: {pay_diff:?}")
            }
            FailedTransactionNonce(failed_diff) => {
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
    use super::{
        AccountDiff, CoinbaseDiff, DelegationDiff, FailedTransactionNonceDiff, PaymentDiff,
        UpdateType,
    };
    use crate::{
        block::precomputed::{PcbVersion, PrecomputedBlock},
        command::{Command, Delegation, Payment},
        constants::MAINNET_COINBASE_REWARD,
        ledger::{
            account::Nonce,
            amount::Amount,
            coinbase::{Coinbase, CoinbaseFeeTransfer, CoinbaseKind},
            diff::LedgerDiff,
            PublicKey,
        },
    };
    use std::path::PathBuf;

    #[test]
    fn test_amount() {
        let credit_amount = Amount(1000);
        let debit_amount = Amount(500);

        // Test Credit for PaymentDiff
        let payment_diff_credit = AccountDiff::Payment(PaymentDiff {
            public_key: PublicKey::new("B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG"),
            amount: credit_amount,
            signed_amount: None,
            update_type: UpdateType::Credit,
        });
        assert_eq!(payment_diff_credit.amount(), 1000);

        // Test Debit for PaymentDiff
        let payment_diff_debit = AccountDiff::Payment(PaymentDiff {
            public_key: PublicKey::new("B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG"),
            amount: debit_amount,
            signed_amount: None,
            update_type: UpdateType::Debit(Some(Nonce(1))),
        });
        assert_eq!(payment_diff_debit.amount(), -500);

        // Test Credit for CoinbaseDiff
        let coinbase_diff = AccountDiff::Coinbase(CoinbaseDiff {
            public_key: PublicKey::new("B62qjoDXHMPZx8AACUrdaKVyDcn7uxbym1kxodgMXztn6iJC2yqEKbs"),
            amount: credit_amount,
        });
        assert_eq!(coinbase_diff.amount(), 1000);

        // Test Credit for FeeTransfer PaymentDiff
        let fee_transfer_diff_credit = AccountDiff::FeeTransfer(PaymentDiff {
            public_key: PublicKey::new("B62qkMUJyt7LmPnfu8in6qshaQSvTgLgNjx6h7YySRJ28wJegJ82n6u"),
            amount: credit_amount,
            signed_amount: None,
            update_type: UpdateType::Credit,
        });
        assert_eq!(fee_transfer_diff_credit.amount(), 1000);

        // Test Debit for FeeTransferViaCoinbase PaymentDiff
        let fee_transfer_via_coinbase_diff_debit =
            AccountDiff::FeeTransferViaCoinbase(PaymentDiff {
                public_key: PublicKey::new(
                    "B62qkMUJyt7LmPnfu8in6qshaQSvTgLgNjx6h7YySRJ28wJegJ82n6u",
                ),
                amount: debit_amount,
                signed_amount: None,
                update_type: UpdateType::Debit(None),
            });
        assert_eq!(fee_transfer_via_coinbase_diff_debit.amount(), -500);

        let delegation_diff = AccountDiff::Delegation(DelegationDiff {
            nonce: Nonce(42),
            delegator: PublicKey::new("B62qpYZ5BUaXq7gkUksirDA5c7okVMBY6VrQbj7YHLARWiBvu6A2fqi"),
            delegate: PublicKey::new("B62qjSytpSK7aEauBprjXDSZwc9ai4YMv9tpmXLQK14Vy941YV36rMz"),
        });
        assert_eq!(delegation_diff.amount(), 0);

        let failed_tx_nonce_diff =
            AccountDiff::FailedTransactionNonce(FailedTransactionNonceDiff {
                public_key: PublicKey::new(
                    "B62qpYZ5BUaXq7gkUksirDA5c7okVMBY6VrQbj7YHLARWiBvu6A2fqi",
                ),
                nonce: Nonce(10),
            });
        assert_eq!(failed_tx_nonce_diff.amount(), 0);
    }

    #[test]
    fn test_fee_transfer_via_coinbase() {
        let fee = 10000000;
        let receiver: PublicKey = "B62qkMUJyt7LmPnfu8in6qshaQSvTgLgNjx6h7YySRJ28wJegJ82n6u".into();
        let snarker: PublicKey = "B62qospDjUj43x2yMKiNehojWWRUsE1wpdUDVpfxH8V3n5Y1QgJKFfw".into();
        let account_diff = AccountDiff::from_coinbase(Coinbase {
            supercharge: true,
            is_new_account: true,
            receiver: receiver.clone(),
            receiver_balance: Some(1440 * (1e9 as u64)),
            kind: CoinbaseKind::Coinbase(Some(CoinbaseFeeTransfer {
                receiver_pk: snarker.clone(),
                fee,
            })),
        });
        let expected_account_diff = vec![
            vec![AccountDiff::Coinbase(CoinbaseDiff {
                public_key: receiver.clone(),
                amount: Amount(1440 * (1e9 as u64)),
            })],
            vec![
                AccountDiff::FeeTransferViaCoinbase(PaymentDiff {
                    public_key: snarker,
                    amount: fee.into(),
                    signed_amount: None,
                    update_type: UpdateType::Credit,
                }),
                AccountDiff::FeeTransferViaCoinbase(PaymentDiff {
                    public_key: receiver,
                    amount: fee.into(),
                    signed_amount: None,
                    update_type: UpdateType::Debit(None),
                }),
            ],
        ];
        assert_eq!(account_diff, expected_account_diff);
    }

    // mainnet-220897-3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw for all
    // tests below
    #[test]
    fn test_from_command() {
        let source_public_key =
            PublicKey::from("B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG");
        let receiver_public_key =
            PublicKey::from("B62qjoDXHMPZx8AACUrdaKVyDcn7uxbym1kxodgMXztn6iJC2yqEKbs");
        let nonce = Nonce(5);
        let payment_command = Command::Payment(Payment {
            source: source_public_key.clone(),
            receiver: receiver_public_key.clone(),
            amount: Amount(536900000000),
            is_new_receiver_account: true,
            nonce,
        });
        let expected_result = vec![vec![
            AccountDiff::Payment(PaymentDiff {
                public_key: receiver_public_key.clone(),
                amount: Amount(536900000000),
                signed_amount: None,
                update_type: UpdateType::Credit,
            }),
            AccountDiff::Payment(PaymentDiff {
                public_key: source_public_key,
                amount: Amount(536900000000),
                signed_amount: None,
                update_type: UpdateType::Debit(Some(nonce + 1)),
            }),
        ]];
        assert_eq!(AccountDiff::from_command(payment_command), expected_result);
    }

    #[test]
    fn test_from_command_delegation() {
        let delegator_public_key =
            PublicKey::from("B62qpYZ5BUaXq7gkUksirDA5c7okVMBY6VrQbj7YHLARWiBvu6A2fqi");
        let delegate_public_key =
            PublicKey::from("B62qjSytpSK7aEauBprjXDSZwc9ai4YMv9tpmXLQK14Vy941YV36rMz");
        let nonce = Nonce(42);
        let delegation_command = Command::Delegation(Delegation {
            delegator: delegator_public_key.clone(),
            delegate: delegate_public_key.clone(),
            nonce,
        });
        let expected_result = vec![vec![AccountDiff::Delegation(DelegationDiff {
            delegator: delegator_public_key,
            delegate: delegate_public_key,
            nonce: nonce + 1,
        })]];
        assert_eq!(
            AccountDiff::from_command(delegation_command),
            expected_result
        );
    }

    #[test]
    fn test_from_coinbase() {
        let receiver = PublicKey::from("B62qospDjUj43x2yMKiNehojWWRUsE1wpdUDVpfxH8V3n5Y1QgJKFfw");
        let account_diff = AccountDiff::from_coinbase(Coinbase {
            supercharge: true,
            is_new_account: false,
            receiver_balance: None,
            receiver: receiver.clone(),
            kind: CoinbaseKind::Coinbase(None),
        });
        let expected_account_diff = vec![vec![AccountDiff::Coinbase(CoinbaseDiff {
            public_key: receiver.clone(),
            amount: Amount(1440 * (1e9 as u64)),
        })]];
        assert_eq!(account_diff, expected_account_diff);
    }

    #[test]
    fn test_public_key_payment() {
        let nonce = Nonce(42);
        let payment_diff = PaymentDiff {
            public_key: PublicKey::new("B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG"),
            amount: Amount(536900000000),
            signed_amount: None,
            update_type: UpdateType::Debit(Some(nonce)),
        };
        let account_diff = AccountDiff::Payment(payment_diff);
        let result = account_diff.public_key();
        let expected = PublicKey::new("B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_public_key_delegation() {
        let delegator = PublicKey::from("B62qpYZ5BUaXq7gkUksirDA5c7okVMBY6VrQbj7YHLARWiBvu6A2fqi");
        let delegation_diff = DelegationDiff {
            nonce: Nonce(42),
            delegator: delegator.clone(),
            delegate: PublicKey::from("B62qjSytpSK7aEauBprjXDSZwc9ai4YMv9tpmXLQK14Vy941YV36rMz"),
        };
        let account_diff = AccountDiff::Delegation(delegation_diff);
        let result = account_diff.public_key();
        assert_eq!(result, delegator);
    }

    #[test]
    fn test_snark_account_creation_deduction() -> anyhow::Result<()> {
        use crate::ledger::diff::AccountDiffType::*;
        let path = PathBuf::from("./tests/data/misc_blocks/mainnet-128743-3NLmYZD9eaV58opgC5RzQXaoPbyC15McNxw1CuCNatj7F9vGBbNz.json");
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V1)?;
        let mut ledger_diff = LedgerDiff::from_precomputed(&block);
        let mut expect_diffs = LedgerDiff::from(&[
            (
                "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy",
                "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM",
                Payment(Nonce(180447)),
                1000,
            ),
            (
                "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy",
                "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM",
                Payment(Nonce(180448)),
                1000,
            ),
            (
                "B62qkofKBUonysS9kvTM2q42P7qR1opURprUuGjPbuuifrPyi61Paob",
                "B62qkofKBUonysS9kvTM2q42P7qR1opURprUuGjPbuuifrPyi61Paob",
                Coinbase,
                MAINNET_COINBASE_REWARD,
            ),
            (
                "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy",
                "B62qkofKBUonysS9kvTM2q42P7qR1opURprUuGjPbuuifrPyi61Paob",
                FeeTransfer,
                2000000,
            ),
            (
                "B62qkofKBUonysS9kvTM2q42P7qR1opURprUuGjPbuuifrPyi61Paob",
                "B62qqsMmiJPjodmXxZuvXpEYRv4sBQLFDz1aHYesVmybTqyfZzWnd2n",
                FeeTransferViaCoinbase,
                1e9 as u64,
            ),
        ]);
        expect_diffs.sort();
        ledger_diff.account_diffs.sort();

        for (i, diff) in ledger_diff.account_diffs.iter().enumerate() {
            assert_eq!(
                *diff, expect_diffs[i],
                "{i}th diff mismatch\n{:#?}\n{:#?}",
                ledger_diff.account_diffs, expect_diffs,
            );
        }
        assert_eq!(ledger_diff.account_diffs, expect_diffs);
        Ok(())
    }
}
