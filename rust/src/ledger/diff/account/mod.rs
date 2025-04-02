//! Ledger account diff representation

pub mod zkapp;

use crate::{
    base::{nonce::Nonce, state_hash::StateHash},
    block::precomputed::PrecomputedBlock,
    command::{Command, CommandWithStateHash, UserCommandWithStatus, UserCommandWithStatusT},
    ledger::{
        coinbase::Coinbase,
        token::{account::TokenAccount, TokenAddress},
        Amount, PublicKey,
    },
    mina_blocks::v2::{
        self,
        protocol_state::SupplyAdjustmentSign,
        staged_ledger_diff::{Authorization, Call, Elt, Precondition, UpdateKind},
    },
    snark_work::SnarkWorkSummary,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use zkapp::{
    ZkappAccountCreationFee, ZkappActionsDiff, ZkappDiff, ZkappEventsDiff, ZkappFeePayerNonceDiff,
    ZkappIncrementNonce, ZkappPaymentDiff, ZkappPermissionsDiff, ZkappProvedStateDiff,
    ZkappStateDiff, ZkappTimingDiff, ZkappTokenSymbolDiff, ZkappUriDiff, ZkappVerificationKeyDiff,
    ZkappVotingForDiff,
};

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

    // All zkapp diffs
    Zkapp(Box<ZkappDiff>),
    ZkappStateDiff(ZkappStateDiff),
    ZkappPermissionsDiff(ZkappPermissionsDiff),
    ZkappVerificationKeyDiff(ZkappVerificationKeyDiff),
    ZkappProvedStateDiff(ZkappProvedStateDiff),
    ZkappUriDiff(ZkappUriDiff),
    ZkappTokenSymbolDiff(ZkappTokenSymbolDiff),
    ZkappTimingDiff(ZkappTimingDiff),
    ZkappVotingForDiff(ZkappVotingForDiff),
    ZkappActionsDiff(ZkappActionsDiff),
    ZkappEventsDiff(ZkappEventsDiff),
    ZkappIncrementNonce(ZkappIncrementNonce),
    ZkappAccountCreationFee(ZkappAccountCreationFee),
    ZkappFeePayerNonce(ZkappFeePayerNonceDiff),
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

    // All zkapp diffs
    Zkapp(Box<ZkappDiff>),
    ZkappStateDiff(ZkappStateDiff),
    ZkappPermissionsDiff(ZkappPermissionsDiff),
    ZkappVerificationKeyDiff(ZkappVerificationKeyDiff),
    ZkappProvedStateDiff(ZkappProvedStateDiff),
    ZkappUriDiff(ZkappUriDiff),
    ZkappTokenSymbolDiff(ZkappTokenSymbolDiff),
    ZkappTimingDiff(ZkappTimingDiff),
    ZkappVotingForDiff(ZkappVotingForDiff),
    ZkappActionsDiff(ZkappActionsDiff),
    ZkappEventsDiff(ZkappEventsDiff),
    ZkappIncrementNonce(ZkappIncrementNonce),
    ZkappAccountCreationFee(ZkappAccountCreationFee),
    ZkappFeePayerNonce(ZkappFeePayerNonceDiff),
}

/// A debit carries Some(nonce) for a payment/delegation command, None for
/// internal commands and zkapp account updates
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Serialize, Deserialize)]
pub enum UpdateType {
    Debit(Option<Nonce>),
    Credit,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct PaymentDiff {
    pub update_type: UpdateType,
    pub public_key: PublicKey,
    pub amount: Amount,
    pub token: TokenAddress,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct DelegationDiff {
    pub nonce: Nonce,
    pub delegator: PublicKey,
    pub delegate: PublicKey,
}

// internal command diffs

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

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum AccountDiffType {
    Payment(Nonce),
    Delegation(Nonce),
    Coinbase,
    FeeTransfer,
    FeeTransferViaCoinbase,
    Zkapp { token: TokenAddress, nonce: Nonce },
}

///////////
// impls //
///////////

impl TokenAccount for AccountDiff {
    fn public_key(&self) -> PublicKey {
        match self {
            Self::Payment(diff) => diff.public_key.clone(),
            Self::Delegation(diff) => diff.delegator.clone(),
            Self::Coinbase(diff) => diff.public_key.clone(),
            Self::FeeTransfer(diff) => diff.public_key.clone(),
            Self::FeeTransferViaCoinbase(diff) => diff.public_key.clone(),
            Self::FailedTransactionNonce(diff) => diff.public_key.clone(),
            Self::Zkapp(diff) => diff.public_key.clone(),
            Self::ZkappStateDiff(diff) => diff.public_key.clone(),
            Self::ZkappPermissionsDiff(diff) => diff.public_key.clone(),
            Self::ZkappVerificationKeyDiff(diff) => diff.public_key.clone(),
            Self::ZkappProvedStateDiff(diff) => diff.public_key.clone(),
            Self::ZkappUriDiff(diff) => diff.public_key.clone(),
            Self::ZkappTokenSymbolDiff(diff) => diff.public_key.clone(),
            Self::ZkappTimingDiff(diff) => diff.public_key.clone(),
            Self::ZkappVotingForDiff(diff) => diff.public_key.clone(),
            Self::ZkappActionsDiff(diff) => diff.public_key.clone(),
            Self::ZkappEventsDiff(diff) => diff.public_key.clone(),
            Self::ZkappIncrementNonce(diff) => diff.public_key.clone(),
            Self::ZkappAccountCreationFee(diff) => diff.public_key.clone(),
            Self::ZkappFeePayerNonce(diff) => diff.public_key.clone(),
        }
    }

    fn token(&self) -> TokenAddress {
        match self {
            Self::Delegation(_)
            | Self::Coinbase(_)
            | Self::FeeTransferViaCoinbase(_)
            | Self::FeeTransfer(_)
            | Self::FailedTransactionNonce(_)
            | Self::ZkappFeePayerNonce(_) => TokenAddress::default(),
            Self::Payment(diff) => diff.token.clone(),
            Self::Zkapp(diff) => diff.token.clone(),
            Self::ZkappStateDiff(diff) => diff.token.clone(),
            Self::ZkappPermissionsDiff(diff) => diff.token.clone(),
            Self::ZkappVerificationKeyDiff(diff) => diff.token.clone(),
            Self::ZkappProvedStateDiff(diff) => diff.token.clone(),
            Self::ZkappUriDiff(diff) => diff.token.clone(),
            Self::ZkappTokenSymbolDiff(diff) => diff.token.clone(),
            Self::ZkappTimingDiff(diff) => diff.token.clone(),
            Self::ZkappVotingForDiff(diff) => diff.token.clone(),
            Self::ZkappActionsDiff(diff) => diff.token.clone(),
            Self::ZkappEventsDiff(diff) => diff.token.clone(),
            Self::ZkappIncrementNonce(diff) => diff.token.clone(),
            Self::ZkappAccountCreationFee(diff) => diff.token.clone(),
        }
    }
}

impl AccountDiff {
    pub fn from_command(command: CommandWithStateHash) -> Vec<Vec<Self>> {
        let CommandWithStateHash {
            command,
            state_hash,
        } = command;
        match command {
            Command::Payment(payment) => {
                vec![vec![
                    Self::Payment(PaymentDiff {
                        public_key: payment.receiver,
                        amount: payment.amount,
                        update_type: UpdateType::Credit,
                        token: TokenAddress::default(), // always MINA
                    }),
                    Self::Payment(PaymentDiff {
                        public_key: payment.source,
                        amount: payment.amount,
                        update_type: UpdateType::Debit(Some(payment.nonce + 1)),
                        token: TokenAddress::default(), // always MINA
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
            Command::Zkapp(zkapp) => {
                let mut diffs = vec![AccountDiff::ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                    state_hash: state_hash.to_owned(),
                    public_key: zkapp.fee_payer.body.public_key.to_owned(),
                    nonce: zkapp.fee_payer.body.nonce + 1,
                })];

                zkapp.account_updates.iter().for_each(|update| {
                    let fee_payer = zkapp.fee_payer.body.public_key.to_owned();
                    let nonce = zkapp.fee_payer.body.nonce;

                    diffs.push(AccountDiff::from_zkapp_account_update(
                        fee_payer.to_owned(),
                        Some(nonce),
                        &update.elt,
                        state_hash.to_owned(),
                    ));

                    recurse_calls(&mut diffs, update.elt.calls.iter(), state_hash.to_owned());
                });

                vec![diffs]
            }
        }
    }

    fn expand_diff(self) -> Vec<Self> {
        match self {
            Self::Zkapp(diff) => diff.expand(),
            _ => vec![self],
        }
    }

    pub fn expand(diffs: Vec<Vec<Self>>) -> Vec<Vec<Self>> {
        diffs
            .into_iter()
            .map(|diffs| diffs.into_iter().flat_map(Self::expand_diff).collect())
            .collect()
    }

    pub fn unapply(self) -> UnapplyAccountDiff {
        use UnapplyAccountDiff::*;

        match self {
            Self::Coinbase(diff) => Coinbase(diff),
            Self::Payment(diff) => Payment(diff),
            Self::FeeTransfer(diff) => FeeTransfer(diff),
            Self::FeeTransferViaCoinbase(diff) => FeeTransferViaCoinbase(diff),
            Self::Delegation(diff) => Delegation(diff),
            Self::FailedTransactionNonce(diff) => FailedTransactionNonce(diff),
            Self::Zkapp(diff) => Zkapp(diff),
            Self::ZkappStateDiff(diff) => ZkappStateDiff(diff),
            Self::ZkappPermissionsDiff(diff) => ZkappPermissionsDiff(diff),
            Self::ZkappVerificationKeyDiff(diff) => ZkappVerificationKeyDiff(diff),
            Self::ZkappProvedStateDiff(diff) => ZkappProvedStateDiff(diff),
            Self::ZkappUriDiff(diff) => ZkappUriDiff(diff),
            Self::ZkappTokenSymbolDiff(diff) => ZkappTokenSymbolDiff(diff),
            Self::ZkappTimingDiff(diff) => ZkappTimingDiff(diff),
            Self::ZkappVotingForDiff(diff) => ZkappVotingForDiff(diff),
            Self::ZkappActionsDiff(diff) => ZkappActionsDiff(diff),
            Self::ZkappEventsDiff(diff) => ZkappEventsDiff(diff),
            Self::ZkappIncrementNonce(diff) => ZkappIncrementNonce(diff),
            Self::ZkappAccountCreationFee(diff) => ZkappAccountCreationFee(diff),
            Self::ZkappFeePayerNonce(diff) => ZkappFeePayerNonce(diff),
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

    fn transaction_fees(
        coinbase_receiver: &PublicKey,
        user_cmds: Vec<UserCommandWithStatus>,
    ) -> Vec<Vec<Self>> {
        let mut fee_map = HashMap::new();

        for user_cmd in user_cmds.iter() {
            let fee = user_cmd.fee();
            fee_map
                .entry(user_cmd.fee_payer_pk())
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
                            update_type: UpdateType::Credit,
                            token: TokenAddress::default(), // always MINA
                        }),
                        Self::FeeTransfer(PaymentDiff {
                            public_key: pk.clone(),
                            amount: (*fee).into(),
                            update_type: UpdateType::Debit(None),
                            token: TokenAddress::default(), // always MINA
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
                if total_fee.0 > 0 {
                    res.push(vec![
                        AccountDiff::FeeTransfer(PaymentDiff {
                            public_key: prover.clone(),
                            amount: *total_fee,
                            update_type: UpdateType::Credit,
                            token: TokenAddress::default(), // always MINA
                        }),
                        AccountDiff::FeeTransfer(PaymentDiff {
                            public_key: precomputed_block.coinbase_receiver(),
                            amount: *total_fee,
                            update_type: UpdateType::Debit(None),
                            token: TokenAddress::default(), // always MINA
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
        use AccountDiff::*;

        match self {
            Delegation(_) | FailedTransactionNonce(_) => 0,
            Coinbase(diff) => diff.amount.0 as i64,
            FeeTransfer(diff) | FeeTransferViaCoinbase(diff) | Payment(diff) => {
                match diff.update_type {
                    UpdateType::Credit => diff.amount.0 as i64,
                    UpdateType::Debit(_) => 0 - diff.amount.0 as i64,
                }
            }
            ZkappAccountCreationFee(diff) => diff.amount.0 as i64,
            Zkapp(_)
            | ZkappStateDiff(_)
            | ZkappPermissionsDiff(_)
            | ZkappVerificationKeyDiff(_)
            | ZkappProvedStateDiff(_)
            | ZkappUriDiff(_)
            | ZkappTokenSymbolDiff(_)
            | ZkappTimingDiff(_)
            | ZkappVotingForDiff(_)
            | ZkappActionsDiff(_)
            | ZkappEventsDiff(_)
            | ZkappIncrementNonce(_)
            | ZkappFeePayerNonce(_) => {
                unreachable!("zkapp commands do not have an amount")
            }
        }
    }

    pub fn add_token_accounts(
        &self,
        zkapp_token_accounts: &mut HashSet<(PublicKey, TokenAddress)>,
    ) {
        use AccountDiff::*;
        match self {
            Zkapp(zkapp) => {
                for diff in zkapp.clone().expand() {
                    zkapp_token_accounts.insert((diff.public_key(), diff.token()));
                }
            }
            ZkappStateDiff(diff) => {
                zkapp_token_accounts.insert((diff.public_key.to_owned(), diff.token.to_owned()));
            }
            ZkappPermissionsDiff(diff) => {
                zkapp_token_accounts.insert((diff.public_key.to_owned(), diff.token.to_owned()));
            }
            ZkappVerificationKeyDiff(diff) => {
                zkapp_token_accounts.insert((diff.public_key.to_owned(), diff.token.to_owned()));
            }
            ZkappUriDiff(diff) => {
                zkapp_token_accounts.insert((diff.public_key.to_owned(), diff.token.to_owned()));
            }
            ZkappTokenSymbolDiff(diff) => {
                zkapp_token_accounts.insert((diff.public_key.to_owned(), diff.token.to_owned()));
            }
            ZkappTimingDiff(diff) => {
                zkapp_token_accounts.insert((diff.public_key.to_owned(), diff.token.to_owned()));
            }
            ZkappVotingForDiff(diff) => {
                zkapp_token_accounts.insert((diff.public_key.to_owned(), diff.token.to_owned()));
            }
            ZkappActionsDiff(diff) => {
                zkapp_token_accounts.insert((diff.public_key.to_owned(), diff.token.to_owned()));
            }
            ZkappEventsDiff(diff) => {
                zkapp_token_accounts.insert((diff.public_key.to_owned(), diff.token.to_owned()));
            }
            ZkappIncrementNonce(diff) => {
                zkapp_token_accounts.insert((diff.public_key.to_owned(), diff.token.to_owned()));
            }
            ZkappAccountCreationFee(diff) => {
                zkapp_token_accounts.insert((diff.public_key.to_owned(), diff.token.to_owned()));
            }
            _ => (),
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
                    update_type: UpdateType::Credit,
                    token: TokenAddress::default(),
                }),
                Self::Payment(PaymentDiff {
                    public_key: sender.into(),
                    amount: amount.into(),
                    update_type: UpdateType::Debit(Some(nonce)),
                    token: TokenAddress::default(),
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
                    update_type: UpdateType::Credit,
                    token: TokenAddress::default(),
                }),
                Self::FeeTransfer(PaymentDiff {
                    public_key: sender.into(),
                    amount: amount.into(),
                    update_type: UpdateType::Debit(None),
                    token: TokenAddress::default(),
                }),
            ]],
            AccountDiffType::FeeTransferViaCoinbase => vec![vec![
                Self::FeeTransferViaCoinbase(PaymentDiff {
                    public_key: receiver.into(),
                    amount: amount.into(),
                    update_type: UpdateType::Credit,
                    token: TokenAddress::default(),
                }),
                Self::FeeTransferViaCoinbase(PaymentDiff {
                    public_key: sender.into(),
                    amount: amount.into(),
                    update_type: UpdateType::Debit(None),
                    token: TokenAddress::default(),
                }),
            ]],
            AccountDiffType::Zkapp { token, nonce } => {
                vec![vec![Self::Zkapp(Box::new(ZkappDiff {
                    state_hash: StateHash::default(),
                    nonce: Some(nonce),
                    token: token.to_owned(),
                    public_key: sender.into(),
                    payment_diffs: vec![
                        ZkappPaymentDiff::Payment {
                            state_hash: StateHash::default(),
                            payment: PaymentDiff {
                                update_type: UpdateType::Credit,
                                public_key: receiver.into(),
                                amount: amount.into(),
                                token: token.to_owned(),
                            },
                        },
                        ZkappPaymentDiff::Payment {
                            state_hash: StateHash::default(),
                            payment: PaymentDiff {
                                update_type: UpdateType::Debit(Some(nonce)),
                                public_key: sender.into(),
                                amount: amount.into(),
                                token,
                            },
                        },
                    ],
                    ..Default::default()
                }))]]
            }
        }
    }
}

impl PaymentDiff {
    pub fn from_account_diff(diff: AccountDiff) -> Vec<Self> {
        use AccountDiff::*;

        match diff {
            Zkapp(diff) => diff
                .payment_diffs
                .into_iter()
                .filter_map(|diff| match diff {
                    ZkappPaymentDiff::Payment { payment, .. } => Some(payment),
                    ZkappPaymentDiff::IncrementNonce(_)
                    | ZkappPaymentDiff::AccountCreationFee(_) => None,
                })
                .collect(),
            Payment(diff) | FeeTransfer(diff) | FeeTransferViaCoinbase(diff) => vec![diff],
            Coinbase(cb_diff) => vec![Self {
                update_type: UpdateType::Credit,
                public_key: cb_diff.public_key,
                amount: cb_diff.amount,
                token: TokenAddress::default(), // always MINA
            }],
            Delegation(_)
            | FailedTransactionNonce(_)
            | ZkappStateDiff(_)
            | ZkappPermissionsDiff(_)
            | ZkappVerificationKeyDiff(_)
            | ZkappProvedStateDiff(_)
            | ZkappUriDiff(_)
            | ZkappTokenSymbolDiff(_)
            | ZkappTimingDiff(_)
            | ZkappVotingForDiff(_)
            | ZkappActionsDiff(_)
            | ZkappEventsDiff(_)
            | ZkappIncrementNonce(_)
            | ZkappAccountCreationFee(_)
            | ZkappFeePayerNonce(_) => vec![],
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

/////////////////
// conversions //
/////////////////

impl From<SupplyAdjustmentSign> for UpdateType {
    fn from(value: SupplyAdjustmentSign) -> Self {
        match value {
            SupplyAdjustmentSign::Neg => Self::Debit(None),
            SupplyAdjustmentSign::Pos => Self::Credit,
        }
    }
}

// zkapp account diffs
impl AccountDiff {
    fn from_zkapp_account_update(
        pk: PublicKey,
        nonce: Option<Nonce>,
        elt: &Elt,
        state_hash: StateHash,
    ) -> Self {
        let fee_payer = pk.to_owned();
        let public_key = elt.account_update.body.public_key.to_owned();

        let mut payment_diffs = vec![];
        let token = elt.account_update.body.token_id.to_owned();

        // token payments
        let amount = elt.account_update.body.balance_change.magnitude.0.into();

        // pay creation fee of receiver zkapp
        use ZkappPaymentDiff::*;
        if elt.account_update.body.implicit_account_creation_fee {
            payment_diffs.push(Payment {
                state_hash: state_hash.to_owned(),
                payment: PaymentDiff {
                    public_key: fee_payer.to_owned(),
                    update_type: UpdateType::Debit(None),
                    token: token.to_owned(),
                    amount,
                },
            });
        }

        // increment nonce of updated account
        let increment_nonce = elt.account_update.body.increment_nonce;
        if increment_nonce {
            payment_diffs.push(IncrementNonce(ZkappIncrementNonce {
                state_hash: state_hash.to_owned(),
                public_key: public_key.to_owned(),
                token: token.to_owned(),
            }));
        };

        // only push non-zero payments
        if amount.0 != 0 {
            payment_diffs.push(Payment {
                state_hash: state_hash.to_owned(),
                payment: PaymentDiff {
                    public_key: public_key.to_owned(),
                    token: token.to_owned(),
                    amount,
                    update_type: elt
                        .account_update
                        .body
                        .balance_change
                        .sgn
                        .0
                        .to_owned()
                        .into(),
                },
            });
        }

        // delegation change
        let delegate = elt.account_update.body.update.delegate.to_owned().into();

        // verification key change
        let verification_key = elt
            .account_update
            .body
            .update
            .verification_key
            .to_owned()
            .into();

        // permissions change
        let permissions = {
            let permissions: Option<v2::Permissions> =
                elt.account_update.body.update.permissions.to_owned().into();
            permissions.map(Into::into)
        };

        // zkapp uri change
        let zkapp_uri = elt.account_update.body.update.zkapp_uri.to_owned().into();

        // token symbol change
        let token_symbol = elt
            .account_update
            .body
            .update
            .token_symbol
            .to_owned()
            .into();

        // account timing change
        let timing = {
            let timing: Option<v2::Timing> =
                elt.account_update.body.update.timing.to_owned().into();
            timing.map(Into::into)
        };

        // account `voting_for` change
        let voting_for = elt.account_update.body.update.voting_for.to_owned().into();

        // update actions
        let actions = if let Some(actions) = elt.account_update.body.actions.first() {
            actions.0.iter().cloned().map(Into::into).collect()
        } else {
            vec![]
        };

        // update events
        let events = if let Some(events) = elt.account_update.body.events.first() {
            events.0.iter().cloned().map(Into::into).collect()
        } else {
            vec![]
        };

        // update app state
        let app_state_diff = elt.account_update.body.update.app_state.to_owned().map(
            |update_kind| match update_kind {
                UpdateKind::Keep(_) => None,
                UpdateKind::Set((_, state)) => Some(state.into()),
            },
        );

        let proved_state = matches!(
            elt.account_update.body.authorization_kind,
            Authorization::Proof(_) | Authorization::Proof_(_)
        ) && matches!(
            elt.account_update.body.preconditions.account.proved_state,
            Precondition::Check(_)
        );

        Self::Zkapp(Box::new(ZkappDiff {
            state_hash,
            token,
            public_key,
            nonce: nonce.map(|n| n + 1),
            increment_nonce,
            proved_state,
            payment_diffs,
            delegate,
            verification_key,
            permissions,
            zkapp_uri,
            token_symbol,
            timing,
            voting_for,
            actions,
            events,
            app_state_diff,
        }))
    }
}

///////////////////
// debug/display //
///////////////////

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
            Payment(diff) => write!(f, "{:<27}{diff:?}", "Payment:"),
            Delegation(diff) => write!(f, "{:<27}{diff:?}", "Delegation:"),
            Coinbase(diff) => write!(f, "{:<27}{diff:?}", "Coinbase:"),
            FeeTransfer(diff) => write!(f, "{:<27}{diff:?}", "Fee transfer:"),
            FeeTransferViaCoinbase(diff) => {
                write!(f, "{:<27}{diff:?}", "Fee transfer via coinbase:")
            }
            FailedTransactionNonce(diff) => {
                write!(f, "{:<27}{diff:?}", "Failed transaction:")
            }
            Zkapp(diff) => write!(f, "{:<27}{diff:?}", "Zkapp:"),
            ZkappStateDiff(diff) => write!(f, "{:<27}{diff:?}", "ZkappState:"),
            ZkappPermissionsDiff(diff) => write!(f, "{:<27}: {diff:?}", "ZkappPermissions:"),
            ZkappVerificationKeyDiff(diff) => write!(f, "{:<27}{diff:?}", "ZkappVK:"),
            ZkappProvedStateDiff(diff) => write!(f, "{:<27}{diff:?}", "ZkappProvedState:"),
            ZkappUriDiff(diff) => write!(f, "{:<27}{diff:?}", "ZkappUri:"),
            ZkappTokenSymbolDiff(diff) => {
                write!(f, "{:<27}{diff:?}", "ZkappTokenSymbol:")
            }
            ZkappTimingDiff(diff) => write!(f, "{:<27}{diff:?}", "ZkappTiming:"),
            ZkappVotingForDiff(diff) => write!(f, "{:<27}{diff:?}", "ZkappVotingFor:"),
            ZkappActionsDiff(diff) => write!(f, "{:<27}{diff:?}", "ZkappActions:"),
            ZkappEventsDiff(diff) => write!(f, "{:<27}{diff:?}", "ZkappEvents:"),
            ZkappIncrementNonce(diff) => {
                write!(f, "{:<27}{}", "ZkappIncrementNonce:", diff.public_key)
            }
            ZkappAccountCreationFee(diff) => write!(f, "{:<27}{diff:?}", "ZkappAccountCreation:"),
            ZkappFeePayerNonce(diff) => write!(f, "{:<27}{diff:?}", "ZkappFeePayerNonce:"),
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

/////////////
// helpers //
/////////////

fn recurse_calls<'a>(
    diffs: &mut Vec<AccountDiff>,
    calls: impl Iterator<Item = &'a Call>,
    state_hash: StateHash,
) {
    for call in calls {
        diffs.push(AccountDiff::from_zkapp_account_update(
            call.elt.account_update.body.public_key.to_owned(),
            None,
            call.elt.as_ref(),
            state_hash.to_owned(),
        ));

        recurse_calls(diffs, call.elt.calls.iter(), state_hash.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        base::nonce::Nonce,
        block::precomputed::{PcbVersion, PrecomputedBlock},
        command::{Command, Delegation, Payment},
        constants::MAINNET_COINBASE_REWARD,
        ledger::{
            coinbase::{Coinbase, CoinbaseFeeTransfer, CoinbaseKind},
            diff::LedgerDiff,
            token::TokenAddress,
            Amount, PublicKey,
        },
    };
    use std::path::PathBuf;

    #[test]
    fn test_amount() -> anyhow::Result<()> {
        let credit_amount = Amount(1000);
        let debit_amount = Amount(500);

        // Test Credit for PaymentDiff
        let payment_diff_credit = AccountDiff::Payment(PaymentDiff {
            public_key: PublicKey::new("B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG")?,
            amount: credit_amount,
            update_type: UpdateType::Credit,
            token: TokenAddress::default(),
        });
        assert_eq!(payment_diff_credit.amount(), 1000);

        // Test Debit for PaymentDiff
        let payment_diff_debit = AccountDiff::Payment(PaymentDiff {
            public_key: PublicKey::new("B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG")?,
            amount: debit_amount,
            update_type: UpdateType::Debit(Some(Nonce(1))),
            token: TokenAddress::default(),
        });
        assert_eq!(payment_diff_debit.amount(), -500);

        // Test Credit for CoinbaseDiff
        let coinbase_diff = AccountDiff::Coinbase(CoinbaseDiff {
            public_key: PublicKey::new("B62qjoDXHMPZx8AACUrdaKVyDcn7uxbym1kxodgMXztn6iJC2yqEKbs")?,
            amount: credit_amount,
        });
        assert_eq!(coinbase_diff.amount(), 1000);

        // Test Credit for FeeTransfer PaymentDiff
        let fee_transfer_diff_credit = AccountDiff::FeeTransfer(PaymentDiff {
            public_key: PublicKey::new("B62qkMUJyt7LmPnfu8in6qshaQSvTgLgNjx6h7YySRJ28wJegJ82n6u")?,
            amount: credit_amount,
            update_type: UpdateType::Credit,
            token: TokenAddress::default(),
        });
        assert_eq!(fee_transfer_diff_credit.amount(), 1000);

        // Test Debit for FeeTransferViaCoinbase PaymentDiff
        let fee_transfer_via_coinbase_diff_debit =
            AccountDiff::FeeTransferViaCoinbase(PaymentDiff {
                public_key: PublicKey::new(
                    "B62qkMUJyt7LmPnfu8in6qshaQSvTgLgNjx6h7YySRJ28wJegJ82n6u",
                )?,
                amount: debit_amount,
                update_type: UpdateType::Debit(None),
                token: TokenAddress::default(),
            });
        assert_eq!(fee_transfer_via_coinbase_diff_debit.amount(), -500);

        let delegation_diff = AccountDiff::Delegation(DelegationDiff {
            nonce: Nonce(42),
            delegator: PublicKey::new("B62qpYZ5BUaXq7gkUksirDA5c7okVMBY6VrQbj7YHLARWiBvu6A2fqi")?,
            delegate: PublicKey::new("B62qjSytpSK7aEauBprjXDSZwc9ai4YMv9tpmXLQK14Vy941YV36rMz")?,
        });
        assert_eq!(delegation_diff.amount(), 0);

        let failed_tx_nonce_diff =
            AccountDiff::FailedTransactionNonce(FailedTransactionNonceDiff {
                public_key: PublicKey::new(
                    "B62qpYZ5BUaXq7gkUksirDA5c7okVMBY6VrQbj7YHLARWiBvu6A2fqi",
                )?,
                nonce: Nonce(10),
            });

        assert_eq!(failed_tx_nonce_diff.amount(), 0);
        Ok(())
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
            kind: CoinbaseKind::One(Some(CoinbaseFeeTransfer {
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
                    update_type: UpdateType::Credit,
                    token: TokenAddress::default(),
                }),
                AccountDiff::FeeTransferViaCoinbase(PaymentDiff {
                    public_key: receiver,
                    amount: fee.into(),
                    update_type: UpdateType::Debit(None),
                    token: TokenAddress::default(),
                }),
            ],
        ];
        assert_eq!(account_diff, expected_account_diff);
    }

    // mainnet-220897-3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw for all
    // tests below
    #[test]
    fn test_from_command() {
        let nonce = Nonce(5);
        let amount = Amount(536900000000);
        let state_hash = StateHash::default();

        let source_public_key =
            PublicKey::from("B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG");
        let receiver_public_key =
            PublicKey::from("B62qjoDXHMPZx8AACUrdaKVyDcn7uxbym1kxodgMXztn6iJC2yqEKbs");

        let payment_command = CommandWithStateHash {
            state_hash,
            command: Command::Payment(Payment {
                nonce,
                amount,
                source: source_public_key.clone(),
                receiver: receiver_public_key.clone(),
                is_new_receiver_account: true,
            }),
        };

        let expected_result = vec![vec![
            AccountDiff::Payment(PaymentDiff {
                amount,
                public_key: receiver_public_key.clone(),
                update_type: UpdateType::Credit,
                token: TokenAddress::default(),
            }),
            AccountDiff::Payment(PaymentDiff {
                amount,
                public_key: source_public_key,
                update_type: UpdateType::Debit(Some(nonce + 1)),
                token: TokenAddress::default(),
            }),
        ]];

        assert_eq!(AccountDiff::from_command(payment_command), expected_result);
    }

    #[test]
    fn test_from_command_delegation() {
        let nonce = Nonce(42);
        let state_hash = StateHash::default();

        let delegator_public_key =
            PublicKey::from("B62qpYZ5BUaXq7gkUksirDA5c7okVMBY6VrQbj7YHLARWiBvu6A2fqi");
        let delegate_public_key =
            PublicKey::from("B62qjSytpSK7aEauBprjXDSZwc9ai4YMv9tpmXLQK14Vy941YV36rMz");

        let delegation_command = CommandWithStateHash {
            state_hash,
            command: Command::Delegation(Delegation {
                delegator: delegator_public_key.clone(),
                delegate: delegate_public_key.clone(),
                nonce,
            }),
        };

        assert_eq!(
            AccountDiff::from_command(delegation_command),
            vec![vec![AccountDiff::Delegation(DelegationDiff {
                delegator: delegator_public_key,
                delegate: delegate_public_key,
                nonce: nonce + 1,
            })]]
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
            kind: CoinbaseKind::One(None),
        });
        let expected_account_diff = vec![vec![AccountDiff::Coinbase(CoinbaseDiff {
            public_key: receiver.clone(),
            amount: Amount(1440 * (1e9 as u64)),
        })]];
        assert_eq!(account_diff, expected_account_diff);
    }

    #[test]
    fn test_public_key_payment() -> anyhow::Result<()> {
        let nonce = Nonce(42);
        let payment_diff = PaymentDiff {
            public_key: PublicKey::new("B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG")?,
            amount: Amount(536900000000),
            update_type: UpdateType::Debit(Some(nonce)),
            token: TokenAddress::default(),
        };
        let account_diff = AccountDiff::Payment(payment_diff);
        let result = account_diff.public_key();

        let expected = PublicKey::new("B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG")?;
        assert_eq!(result, expected);

        Ok(())
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
