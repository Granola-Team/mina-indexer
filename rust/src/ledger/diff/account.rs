//! Ledger account diff representation

use crate::{
    base::{nonce::Nonce, state_hash::StateHash},
    block::precomputed::PrecomputedBlock,
    command::{Command, CommandWithStateHash, UserCommandWithStatus, UserCommandWithStatusT},
    constants::ZKAPP_STATE_FIELD_ELEMENTS_NUM,
    ledger::{
        account::{Permissions, Timing},
        coinbase::Coinbase,
        token::{account::TokenAccount, TokenAddress, TokenSymbol},
        Amount, PublicKey,
    },
    mina_blocks::v2::{
        self,
        protocol_state::SupplyAdjustmentSign,
        staged_ledger_diff::{Authorization, Call, Elt, UpdateKind},
        zkapp::action_state::ActionState,
        AppState, VerificationKey, ZkappEvent, ZkappUri,
    },
    snark_work::SnarkWorkSummary,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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
    pub amount: Amount,
    pub token: TokenAddress,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct DelegationDiff {
    pub nonce: Nonce,
    pub delegator: PublicKey,
    pub delegate: PublicKey,
}

/////////////////
// zkapp diffs //
/////////////////

/// Aggregated zkapp diff:
/// - make token payments
/// - change app state elements
/// - change delegate
/// - change verification key
/// - change permissions
/// - change zkapp uri
/// - change token symbol
/// - change timing
/// - change voting for
/// - change action state
/// - change events
/// - increment nonces
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappDiff {
    pub state_hash: StateHash,
    pub nonce: Nonce,
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub increment_nonce: bool,
    pub payment_diffs: Vec<ZkappPaymentDiff>,
    pub proved_state: bool,
    pub app_state_diff: [Option<AppState>; ZKAPP_STATE_FIELD_ELEMENTS_NUM],
    pub delegate: Option<PublicKey>,
    pub verification_key: Option<VerificationKey>,
    pub permissions: Option<Permissions>,
    pub zkapp_uri: Option<ZkappUri>,
    pub token_symbol: Option<TokenSymbol>,
    pub timing: Option<Timing>,
    pub voting_for: Option<StateHash>,
    pub actions: Vec<ActionState>,
    pub events: Vec<ZkappEvent>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub enum ZkappPaymentDiff {
    IncrementNonce(ZkappIncrementNonce),
    AccountCreationFee(ZkappAccountCreationFee),
    Payment {
        state_hash: StateHash,
        payment: PaymentDiff,
    },
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappStateDiff {
    pub state_hash: StateHash,
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub proved_state: bool,
    pub diffs: [Option<AppState>; ZKAPP_STATE_FIELD_ELEMENTS_NUM],
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappVerificationKeyDiff {
    pub state_hash: StateHash,
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub proved_state: bool,
    pub verification_key: VerificationKey,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappPermissionsDiff {
    pub state_hash: StateHash,
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub permissions: Permissions,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappUriDiff {
    pub state_hash: StateHash,
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub proved_state: bool,
    pub zkapp_uri: ZkappUri,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappTokenSymbolDiff {
    pub state_hash: StateHash,
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub token_symbol: TokenSymbol,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappTimingDiff {
    pub state_hash: StateHash,
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub timing: Timing,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappVotingForDiff {
    pub state_hash: StateHash,
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub voting_for: StateHash,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappActionsDiff {
    pub state_hash: StateHash,
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub proved_state: bool,
    pub actions: Vec<ActionState>,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappEventsDiff {
    pub state_hash: StateHash,
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub events: Vec<ZkappEvent>,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappIncrementNonce {
    pub state_hash: StateHash,
    pub public_key: PublicKey,
    pub token: TokenAddress,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappAccountCreationFee {
    pub state_hash: StateHash,
    pub public_key: PublicKey,
    pub token: TokenAddress,
    pub amount: Amount,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappFeePayerNonceDiff {
    pub state_hash: StateHash,
    pub public_key: PublicKey,
    pub nonce: Nonce,
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

                    diffs.push(AccountDiff::from_zkapp(
                        fee_payer.to_owned(),
                        nonce,
                        &update.elt,
                        state_hash.to_owned(),
                    ));

                    recurse_calls(
                        &mut diffs,
                        update.elt.calls.iter(),
                        fee_payer,
                        nonce,
                        state_hash.to_owned(),
                    );
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
                    nonce,
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

impl ZkappDiff {
    pub fn expand(self) -> Vec<AccountDiff> {
        let mut account_diffs = vec![];

        // payments
        for payment_diff in self.payment_diffs {
            Self::expand_payment_diff(&mut account_diffs, payment_diff);
        }

        // app state
        Self::expand_app_state_diff(
            &mut account_diffs,
            self.state_hash.to_owned(),
            self.token.to_owned(),
            self.public_key.to_owned(),
            self.proved_state,
            self.app_state_diff,
        );

        // delegate
        Self::expand_delegate_diff(
            &mut account_diffs,
            self.public_key.to_owned(),
            self.nonce,
            self.delegate,
        );

        // verification key
        Self::expand_verification_key_diff(
            &mut account_diffs,
            self.state_hash.to_owned(),
            self.token.to_owned(),
            self.public_key.to_owned(),
            self.proved_state,
            self.verification_key,
        );

        // permissions
        Self::expand_permissions_diff(
            &mut account_diffs,
            self.state_hash.to_owned(),
            self.token.to_owned(),
            self.public_key.to_owned(),
            self.permissions,
        );

        // zkapp uri
        Self::expand_uri_diff(
            &mut account_diffs,
            self.state_hash.to_owned(),
            self.token.to_owned(),
            self.public_key.to_owned(),
            self.proved_state,
            self.zkapp_uri,
        );

        // token symbol
        Self::expand_symbol_diff(
            &mut account_diffs,
            self.state_hash.to_owned(),
            self.token.to_owned(),
            self.public_key.to_owned(),
            self.token_symbol,
        );

        // timing
        Self::expand_timing_diff(
            &mut account_diffs,
            self.state_hash.to_owned(),
            self.token.to_owned(),
            self.public_key.to_owned(),
            self.timing,
        );

        // voting for
        Self::expand_voting_for_diff(
            &mut account_diffs,
            self.state_hash.to_owned(),
            self.token.to_owned(),
            self.public_key.to_owned(),
            self.voting_for,
        );

        // actions
        Self::expand_actions_diff(
            &mut account_diffs,
            self.state_hash.to_owned(),
            self.token.to_owned(),
            self.public_key.to_owned(),
            self.proved_state,
            self.actions,
        );

        // events
        Self::expand_events_diff(
            &mut account_diffs,
            self.state_hash.to_owned(),
            self.token.to_owned(),
            self.public_key.to_owned(),
            self.events,
        );

        account_diffs
    }

    fn expand_payment_diff(account_diffs: &mut Vec<AccountDiff>, diff: ZkappPaymentDiff) {
        use ZkappPaymentDiff::*;

        let acct_diff = match diff {
            Payment { payment, .. } => AccountDiff::Payment(payment),
            IncrementNonce(diff) => AccountDiff::ZkappIncrementNonce(diff),
            AccountCreationFee(diff) => AccountDiff::ZkappAccountCreationFee(diff),
        };

        account_diffs.push(acct_diff)
    }

    fn expand_app_state_diff(
        account_diffs: &mut Vec<AccountDiff>,
        state_hash: StateHash,
        token: TokenAddress,
        pk: PublicKey,
        proved_state: bool,
        app_state_diff: [Option<AppState>; ZKAPP_STATE_FIELD_ELEMENTS_NUM],
    ) {
        if !app_state_diff.iter().all(|state| state.is_none()) {
            account_diffs.push(AccountDiff::ZkappStateDiff(ZkappStateDiff {
                state_hash,
                token,
                public_key: pk,
                proved_state,
                diffs: app_state_diff,
            }));
        }
    }

    fn expand_delegate_diff(
        account_diffs: &mut Vec<AccountDiff>,
        pk: PublicKey,
        nonce: Nonce,
        delegate: Option<PublicKey>,
    ) {
        if let Some(delegate) = delegate {
            account_diffs.push(AccountDiff::Delegation(DelegationDiff {
                nonce,
                delegator: pk,
                delegate,
            }));
        }
    }

    fn expand_verification_key_diff(
        account_diffs: &mut Vec<AccountDiff>,
        state_hash: StateHash,
        token: TokenAddress,
        pk: PublicKey,
        proved_state: bool,
        verification_key: Option<VerificationKey>,
    ) {
        if let Some(verification_key) = verification_key {
            account_diffs.push(AccountDiff::ZkappVerificationKeyDiff(
                ZkappVerificationKeyDiff {
                    state_hash,
                    token,
                    public_key: pk,
                    proved_state,
                    verification_key,
                },
            ));
        }
    }

    fn expand_permissions_diff(
        account_diffs: &mut Vec<AccountDiff>,
        state_hash: StateHash,
        token: TokenAddress,
        pk: PublicKey,
        permissions: Option<Permissions>,
    ) {
        if let Some(permissions) = permissions {
            account_diffs.push(AccountDiff::ZkappPermissionsDiff(ZkappPermissionsDiff {
                state_hash,
                token,
                public_key: pk,
                permissions,
            }));
        }
    }

    fn expand_uri_diff(
        account_diffs: &mut Vec<AccountDiff>,
        state_hash: StateHash,
        token: TokenAddress,
        pk: PublicKey,
        proved_state: bool,
        zkapp_uri: Option<ZkappUri>,
    ) {
        if let Some(zkapp_uri) = zkapp_uri {
            account_diffs.push(AccountDiff::ZkappUriDiff(ZkappUriDiff {
                state_hash,
                token,
                public_key: pk,
                proved_state,
                zkapp_uri,
            }));
        }
    }

    fn expand_symbol_diff(
        account_diffs: &mut Vec<AccountDiff>,
        state_hash: StateHash,
        token: TokenAddress,
        pk: PublicKey,
        token_symbol: Option<TokenSymbol>,
    ) {
        if let Some(token_symbol) = token_symbol {
            account_diffs.push(AccountDiff::ZkappTokenSymbolDiff(ZkappTokenSymbolDiff {
                state_hash,
                token,
                public_key: pk,
                token_symbol,
            }));
        }
    }

    fn expand_timing_diff(
        account_diffs: &mut Vec<AccountDiff>,
        state_hash: StateHash,
        token: TokenAddress,
        pk: PublicKey,
        timing: Option<Timing>,
    ) {
        if let Some(timing) = timing {
            account_diffs.push(AccountDiff::ZkappTimingDiff(ZkappTimingDiff {
                state_hash,
                token,
                public_key: pk,
                timing,
            }));
        }
    }

    fn expand_voting_for_diff(
        account_diffs: &mut Vec<AccountDiff>,
        state_hash: StateHash,
        token: TokenAddress,
        pk: PublicKey,
        voting_for: Option<StateHash>,
    ) {
        if let Some(voting_for) = voting_for {
            account_diffs.push(AccountDiff::ZkappVotingForDiff(ZkappVotingForDiff {
                state_hash,
                token,
                public_key: pk,
                voting_for,
            }));
        }
    }

    fn expand_actions_diff(
        account_diffs: &mut Vec<AccountDiff>,
        state_hash: StateHash,
        token: TokenAddress,
        pk: PublicKey,
        proved_state: bool,
        actions: Vec<ActionState>,
    ) {
        if !actions.is_empty() {
            account_diffs.push(AccountDiff::ZkappActionsDiff(ZkappActionsDiff {
                state_hash,
                token,
                public_key: pk,
                proved_state,
                actions,
            }));
        }
    }

    fn expand_events_diff(
        account_diffs: &mut Vec<AccountDiff>,
        state_hash: StateHash,
        token: TokenAddress,
        pk: PublicKey,
        events: Vec<ZkappEvent>,
    ) {
        if !events.is_empty() {
            account_diffs.push(AccountDiff::ZkappEventsDiff(ZkappEventsDiff {
                state_hash,
                token,
                public_key: pk,
                events,
            }));
        }
    }
}

impl ZkappStateDiff {
    pub fn from_account<T>(public_key: T) -> Self
    where
        T: Into<PublicKey>,
    {
        Self {
            public_key: public_key.into(),
            ..Default::default()
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

impl ZkappPaymentDiff {
    pub fn public_key(&self) -> PublicKey {
        match self {
            Self::Payment { payment, .. } => payment.public_key.to_owned(),
            Self::IncrementNonce(diff) => diff.public_key.to_owned(),
            Self::AccountCreationFee(diff) => diff.public_key.to_owned(),
        }
    }

    pub fn token(&self) -> TokenAddress {
        match self {
            Self::Payment { payment, .. } => payment.token.to_owned(),
            Self::IncrementNonce(diff) => diff.token.to_owned(),
            Self::AccountCreationFee(diff) => diff.token.to_owned(),
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
    fn from_zkapp(pk: PublicKey, nonce: Nonce, elt: &Elt, state_hash: StateHash) -> Self {
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
                    update_type: UpdateType::Debit(Some(nonce + 1)),
                    token: token.to_owned(),
                    amount,
                },
            });
        }

        // increment nonce of receiver
        let increment_nonce = elt.account_update.body.increment_nonce;
        if increment_nonce && amount.0 == 0 {
            payment_diffs.push(IncrementNonce(ZkappIncrementNonce {
                state_hash: state_hash.to_owned(),
                public_key: public_key.to_owned(),
                token: token.to_owned(),
            }));
        };

        let mut update_type: UpdateType = elt
            .account_update
            .body
            .balance_change
            .sgn
            .0
            .to_owned()
            .into();

        if matches!(update_type, UpdateType::Debit(_)) {
            update_type = UpdateType::Debit(Some(nonce + 1))
        }

        // only push non-zero payments
        if amount.0 != 0 {
            payment_diffs.push(Payment {
                state_hash: state_hash.to_owned(),
                payment: PaymentDiff {
                    public_key: public_key.to_owned(),
                    token: token.to_owned(),
                    amount,
                    update_type,
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
        );

        Self::Zkapp(Box::new(ZkappDiff {
            state_hash: state_hash.to_owned(),
            token,
            public_key,
            nonce: nonce + 1,
            increment_nonce,
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
            proved_state,
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

fn recurse_calls<'a>(
    diffs: &mut Vec<AccountDiff>,
    calls: impl Iterator<Item = &'a Call>,
    fee_payer: PublicKey,
    nonce: Nonce,
    state_hash: StateHash,
) {
    for call in calls {
        diffs.push(AccountDiff::from_zkapp(
            fee_payer.to_owned(),
            nonce,
            call.elt.as_ref(),
            state_hash.to_owned(),
        ));

        recurse_calls(
            diffs,
            call.elt.calls.iter(),
            fee_payer.to_owned(),
            nonce,
            state_hash.to_owned(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        base::nonce::Nonce,
        block::precomputed::{PcbVersion, PrecomputedBlock},
        command::{Command, Delegation, Payment},
        constants::{MAINNET_ACCOUNT_CREATION_FEE, MAINNET_COINBASE_REWARD},
        ledger::{
            account::Permission,
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

    #[test]
    #[allow(clippy::too_many_lines)]
    fn zkapp_account_diff() -> anyhow::Result<()> {
        let path = PathBuf::from("./tests/data/misc_blocks/mainnet-359630-3NLjRmTyUzeA7meRAT3Yjqxzfe95GKBgkLPD2iLeVE5RMCFcw8eL.json");
        let pcb = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;

        // zkapp ledger diffs
        let diffs = LedgerDiff::from_precomputed_unexpanded(&pcb);
        let zkapp_diffs = diffs.filter_zkapp();

        // expected unexpanded zkapp account diffs
        let state_hash: StateHash = "3NLjRmTyUzeA7meRAT3Yjqxzfe95GKBgkLPD2iLeVE5RMCFcw8eL".into();
        let expect = vec![
            vec![
                AccountDiff::ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    state_hash: state_hash.to_owned(),
                    nonce: 185.into(),
                }),
                AccountDiff::Zkapp(Box::new(ZkappDiff {
                    nonce: 185.into(),
                    state_hash: state_hash.to_owned(),
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    payment_diffs: vec![ZkappPaymentDiff::Payment {
                        payment: PaymentDiff {
                            public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5"
                                .into(),
                            update_type: UpdateType::Debit(Some(185.into())),
                            amount: 2000000000.into(),
                            token: TokenAddress::default(),
                        },
                        state_hash: state_hash.to_owned(),
                    }],
                    ..Default::default()
                })),
                AccountDiff::Zkapp(Box::new(ZkappDiff {
                    nonce: 185.into(),
                    state_hash: state_hash.to_owned(),
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    payment_diffs: vec![ZkappPaymentDiff::Payment {
                        payment: PaymentDiff {
                            public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5"
                                .into(),
                            update_type: UpdateType::Credit,
                            amount: 2000000000.into(),
                            token: TokenAddress::default(),
                        },
                        state_hash: state_hash.to_owned(),
                    }],
                    ..Default::default()
                })),
            ],
            vec![
                AccountDiff::ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    state_hash: state_hash.to_owned(),
                    nonce: 186.into(),
                }),
                AccountDiff::Zkapp(Box::new(ZkappDiff {
                    nonce: 186.into(),
                    state_hash: state_hash.to_owned(),
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    payment_diffs: vec![ZkappPaymentDiff::Payment {
                        payment: PaymentDiff {
                            public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5"
                                .into(),
                            update_type: UpdateType::Debit(Some(186.into())),
                            amount: 2000000000.into(),
                            token: TokenAddress::default(),
                        },
                        state_hash: state_hash.to_owned()
                    }],
                    ..Default::default()
                })),
                AccountDiff::Zkapp(Box::new(ZkappDiff {
                    nonce: 186.into(),
                    state_hash: state_hash.to_owned(),
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    payment_diffs: vec![ZkappPaymentDiff::Payment {
                        payment: PaymentDiff {
                            public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5"
                                .into(),
                            update_type: UpdateType::Credit,
                            amount: 2000000000.into(),
                            token: TokenAddress::default(),
                        },
                        state_hash: state_hash.to_owned(),
                    }],
                    ..Default::default()
                })),
            ],
            vec![
                AccountDiff::ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    state_hash: state_hash.to_owned(),
                    nonce: 187.into(),
                }),
                AccountDiff::Zkapp(Box::new(ZkappDiff {
                    nonce: 187.into(),
                    state_hash: state_hash.to_owned(),
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    payment_diffs: vec![ZkappPaymentDiff::Payment {
                        payment: PaymentDiff {
                            public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5"
                                .into(),
                            update_type: UpdateType::Debit(Some(187.into())),
                            amount: 2000000000.into(),
                            token: TokenAddress::default(),
                        },
                        state_hash: state_hash.to_owned(),
                    }],
                    ..Default::default()
                })),
                AccountDiff::Zkapp(Box::new(ZkappDiff {
                    nonce: 187.into(),
                    state_hash: state_hash.to_owned(),
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    payment_diffs: vec![ZkappPaymentDiff::Payment {
                        payment: PaymentDiff {
                            public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5"
                                .into(),
                            update_type: UpdateType::Credit,
                            amount: 2000000000.into(),
                            token: TokenAddress::default(),
                        },
                        state_hash: state_hash.to_owned(),
                    }],
                    ..Default::default()
                })),
            ],
            vec![
                AccountDiff::ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                    public_key: "B62qoxZPhqRsKromMF72kjZr6LQnufZ8T2iZuDzCmtuDnnddCRF7fpp".into(),
                    state_hash: state_hash.to_owned(),
                    nonce: 5.into(),
                }),
                AccountDiff::Zkapp(Box::new(ZkappDiff {
                nonce: 5.into(),
                state_hash: state_hash.to_owned(),
                public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".into(),
                payment_diffs: vec![
                    ZkappPaymentDiff::IncrementNonce(ZkappIncrementNonce {
                        state_hash: state_hash.to_owned(),
                        public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
                            .into(),
                        token: TokenAddress::default(),
                    }),
                ],
                verification_key: Some(VerificationKey {
                    data: "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".into(),
                    hash: "0x1C9320E5FD23AF1F8D8B1145484181C3E6B0F1C8C24FE4BDFFEF4281A61C3EBC".into(),
                }),
                permissions: Some(Permissions {
                    edit_state: Permission::Proof,
                    access: Permission::None,
                    send: Permission::Proof,
                    receive: Permission::None,
                    set_delegate: Permission::Signature,
                    set_permissions: Permission::Signature,
                    set_verification_key: (Permission::Signature, "3".to_string()),
                    set_zkapp_uri: Permission::Signature,
                    edit_action_state: Permission::Proof,
                    set_token_symbol: Permission::Signature,
                    increment_nonce: Permission::Signature,
                    set_voting_for: Permission::Signature,
                    set_timing: Permission::Signature
                }),
                zkapp_uri: Some("https://minainu.com".into()),
                token_symbol: Some("MINU".into()),
                increment_nonce: true,
                ..Default::default()
            }))],
        ];

        assert_eq!(zkapp_diffs, expect);

        // expected expanded zkapp diffs
        let expect = vec![
            vec![
                AccountDiff::ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    state_hash: state_hash.to_owned(),
                    nonce: 185.into(),
                }),
                AccountDiff::Payment(PaymentDiff {
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    update_type: UpdateType::Debit(Some(185.into())),
                    amount: 2000000000.into(),
                    token: TokenAddress::default(),
                }),
                AccountDiff::Payment(PaymentDiff {
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    update_type: UpdateType::Credit,
                    amount: 2000000000.into(),
                    token: TokenAddress::default(),
                }),
            ],
            vec![
                AccountDiff::ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    state_hash: state_hash.to_owned(),
                    nonce: 186.into(),
                }),
                AccountDiff::Payment(PaymentDiff {
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    update_type: UpdateType::Debit(Some(186.into())),
                    amount: 2000000000.into(),
                    token: TokenAddress::default(),
                }),
                AccountDiff::Payment(PaymentDiff {
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    update_type: UpdateType::Credit,
                    amount: 2000000000.into(),
                    token: TokenAddress::default(),
                }),
            ],
            vec![
                AccountDiff::ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    state_hash: state_hash.to_owned(),
                    nonce: 187.into(),
                }),
                AccountDiff::Payment(PaymentDiff {
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    update_type: UpdateType::Debit(Some(187.into())),
                    amount: 2000000000.into(),
                    token: TokenAddress::default(),
                }),
                AccountDiff::Payment(PaymentDiff {
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    update_type: UpdateType::Credit,
                    amount: 2000000000.into(),
                    token: TokenAddress::default(),
                }),
            ],
            vec![
                AccountDiff::ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                    public_key: "B62qoxZPhqRsKromMF72kjZr6LQnufZ8T2iZuDzCmtuDnnddCRF7fpp".into(),
                    state_hash: state_hash.to_owned(),
                    nonce: 5.into(),
                }),
                AccountDiff::ZkappIncrementNonce(ZkappIncrementNonce {
                    state_hash: state_hash.to_owned(),
                    public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".into(),
                    token: TokenAddress::default(),
                }),
                AccountDiff::ZkappVerificationKeyDiff(ZkappVerificationKeyDiff {
                    state_hash: state_hash.to_owned(),
                    token: TokenAddress::default(),
                    public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".into(),
                    proved_state: false,
                    verification_key: VerificationKey {
                        data: "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".into(),
                        hash: "0x1C9320E5FD23AF1F8D8B1145484181C3E6B0F1C8C24FE4BDFFEF4281A61C3EBC".into(),
                    }
                }),
                AccountDiff::ZkappPermissionsDiff(ZkappPermissionsDiff {
                    state_hash: state_hash.to_owned(),
                    token: TokenAddress::default(),
                    public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".into(),
                    permissions: Permissions {
                        edit_state: Permission::Proof,
                        access: Permission::None,
                        send: Permission::Proof,
                        receive: Permission::None,
                        set_delegate: Permission::Signature,
                        set_permissions: Permission::Signature,
                        set_verification_key: (Permission::Signature, "3".to_string()),
                        set_zkapp_uri: Permission::Signature,
                        edit_action_state: Permission::Proof,
                        set_token_symbol: Permission::Signature,
                        increment_nonce: Permission::Signature,
                        set_voting_for: Permission::Signature,
                        set_timing: Permission::Signature
                    }
                }),
                AccountDiff::ZkappUriDiff(ZkappUriDiff {
                    state_hash: state_hash.to_owned(),
                    token: TokenAddress::default(),
                    public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".into(),
                    proved_state: false,
                    zkapp_uri: "https://minainu.com".into()
                }),
                AccountDiff::ZkappTokenSymbolDiff(ZkappTokenSymbolDiff {
                    state_hash: state_hash.to_owned(),
                    token: TokenAddress::default(),
                    public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".into(),
                    token_symbol: "MINU".into()
                }),
            ],
        ];

        let expanded = AccountDiff::expand(zkapp_diffs);
        assert_eq!(expanded, expect);

        Ok(())
    }

    #[test]
    fn zkapp_account_diff_new_token() -> anyhow::Result<()> {
        let path = PathBuf::from("./tests/data/misc_blocks/mainnet-360930-3NL3mVAEwJuBS8F3fMWBZZRjQC4JBzdGTD7vN5SqizudnkPKsRyi.json");
        let pcb = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;
        let ledger_diff = LedgerDiff::from_precomputed(&pcb);

        // account creation diffs
        let account_creation_diffs: Vec<_> = ledger_diff
            .new_pk_balances
            .iter()
            .flat_map(|(pk, created)| {
                let mut creation_diffs = vec![];

                for token in created.keys() {
                    creation_diffs.push((pk.to_owned(), token.to_owned()));
                }

                creation_diffs
            })
            .collect();

        assert_eq!(
            account_creation_diffs,
            vec![(
                "B62qnVLedrzTUMZME91WKbNw3qJ3hw7cc5PeK6RR3vH7RTFTsVbiBj4".into(),
                TokenAddress::new("xosVXFFDvDiKvHSDAaHvrTSRtoa5Graf2J7LM5Smb4GNTrT2Hn").unwrap()
            )]
        );

        // account diffs
        let zkapp_diffs = LedgerDiff::from_precomputed_unexpanded(&pcb).filter_zkapp();
        let zkapp_diffs = AccountDiff::expand(zkapp_diffs);

        // expected zkapp account diffs
        let state_hash: StateHash = "3NL3mVAEwJuBS8F3fMWBZZRjQC4JBzdGTD7vN5SqizudnkPKsRyi".into();
        let fee_payer: PublicKey = "B62qo69VLUPMXEC6AFWRgjdTEGsA3xKvqeU5CgYm3jAbBJL7dTvaQkv".into();
        let nonce = Nonce(1);
        let token =
            TokenAddress::new("xosVXFFDvDiKvHSDAaHvrTSRtoa5Graf2J7LM5Smb4GNTrT2Hn").unwrap();

        let expect = vec![vec![
            AccountDiff::ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                state_hash: state_hash.to_owned(),
                public_key: fee_payer.to_owned(),
                nonce: Nonce(1)
            }),
            AccountDiff::Payment(PaymentDiff {
                public_key: fee_payer.clone(),
                update_type: UpdateType::Debit(Some(nonce)),
                amount: 1000000000.into(),
                token: TokenAddress::default(),
            }),
            AccountDiff::ZkappEventsDiff(ZkappEventsDiff {
                state_hash: state_hash.clone(),
                token: TokenAddress::default(),
                public_key: "B62qnzkHunByjReoEwMKCJ9HQxZP2MSYcUe8Lfesy4SpufxWp3viNFT".into(),
                events: vec![
                    "0x0000000000000000000000000000000000000000000000000000000000000002".into(),
                    "0x0000000000000000000000000000000000000000000000000000017473726966".into(),
                    "0x1F290436EC95D83FE3964FEF01CDBAC244EA4D5A7FE99052484DDA0F02C86278".into(),
                    "0x0000000000000000000000000000000000000000000000000000000000000001".into(),
                    "0x000000000000000000000000000000000000000000000000000000174876E800".into(),
                    "0x102580B0AD2B843A16580B526009CD92AD58CDA00675DD8940F2A74E78663DC0".into(),
                    "0x30AF2ED4225D4215C49D1AEE9B7094A0605C0F957219EED3C21CC208119AF5CD".into(),
                    "0x00367A72767777746773677A6C706F6E3563376D6137616965726B6661623A69".into(),
                    "0x0001697767343664727679647636663466683465706A626D73376D776C756F32".into(),
                ]
            }),
            AccountDiff::Payment(PaymentDiff {
                public_key: fee_payer.clone(),
                update_type: UpdateType::Debit(Some(nonce)),
                amount: 19000000000.into(),
                token: TokenAddress::default(),
            }),
            AccountDiff::Payment(PaymentDiff {
                public_key: "B62qq7ecvBQZQK68dwstL27888NEKZJwNXNFjTyu3xpQcfX5UBivCU6".into(),
                update_type: UpdateType::Credit,
                amount: 19000000000.into(),
                token: TokenAddress::default(),
            }),
            AccountDiff::ZkappIncrementNonce(ZkappIncrementNonce {
                state_hash: state_hash.clone(),
                public_key: "B62qnVLedrzTUMZME91WKbNw3qJ3hw7cc5PeK6RR3vH7RTFTsVbiBj4".into(),
                token: token.clone(),
            }),
            AccountDiff::ZkappStateDiff(ZkappStateDiff {
                state_hash: state_hash.clone(),
                token: token.clone(),
                public_key: "B62qnVLedrzTUMZME91WKbNw3qJ3hw7cc5PeK6RR3vH7RTFTsVbiBj4".into(),
                diffs: [
                    Some("0x0000000000000000000000000000000000000000000000000000017473726966".into()),
                    Some("0x102580B0AD2B843A16580B526009CD92AD58CDA00675DD8940F2A74E78663DC0".into()),
                    Some("0x30AF2ED4225D4215C49D1AEE9B7094A0605C0F957219EED3C21CC208119AF5CD".into()),
                    Some("0x00367A72767777746773677A6C706F6E3563376D6137616965726B6661623A69".into()),
                    Some("0x0001697767343664727679647636663466683465706A626D73376D776C756F32".into()),
                    Some("0x29E337A3B00D49D78EAAC1CADEBF4E9278D530F94C9E9259EBCA7B09BFCD4A8A".into()),
                    Some("0x0000000000000000000000000000000000000000000000000000000000000000".into()),
                    Some("0x000000000000000000000000000000000000000000000001000000174876E800".into()),
                ],
                ..Default::default()
            }),
            AccountDiff::ZkappVerificationKeyDiff(ZkappVerificationKeyDiff {
                state_hash: state_hash.clone(),
                token: token.clone(),
                public_key: "B62qnVLedrzTUMZME91WKbNw3qJ3hw7cc5PeK6RR3vH7RTFTsVbiBj4".into(),
                proved_state: false,
                verification_key: VerificationKey {
                    data: "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".into(),
                    hash: "0x3ECC0FC66665B96DA4ED0CC9EF3926C359B0EA44D81E537D02051DAD97F49BED".into()
                },
            }),
            AccountDiff::ZkappPermissionsDiff(ZkappPermissionsDiff {
                state_hash: state_hash.clone(),
                token: token.clone(),
                public_key: "B62qnVLedrzTUMZME91WKbNw3qJ3hw7cc5PeK6RR3vH7RTFTsVbiBj4".into(),
                permissions: Permissions {
                    edit_state: Permission::Proof,
                    access: Permission::None,
                    send: Permission::Proof,
                    receive: Permission::None,
                    set_delegate: Permission::Signature,
                    set_permissions: Permission::Signature,
                    set_verification_key: (Permission::Signature, "3".to_string()),
                    set_zkapp_uri: Permission::Signature,
                    edit_action_state: Permission::Proof,
                    set_token_symbol: Permission::Signature,
                    increment_nonce: Permission::Signature,
                    set_voting_for: Permission::Signature,
                    set_timing: Permission::Signature
                }
            }),
            AccountDiff::Payment(PaymentDiff {
                public_key: "B62qnVLedrzTUMZME91WKbNw3qJ3hw7cc5PeK6RR3vH7RTFTsVbiBj4".into(),
                update_type: UpdateType::Credit,
                amount: Amount(1000000000),
                token: token.clone(),
            }),
        ]];

        for (n, x) in expect[0].iter().enumerate() {
            assert_eq!(zkapp_diffs[0][n], *x, "n = {n}")
        }
        assert_eq!(zkapp_diffs, expect);

        // zkapp accounts created
        let created: Vec<_> = ledger_diff
            .account_diffs
            .into_iter()
            .flatten()
            .filter(|diff| matches!(diff, AccountDiff::ZkappAccountCreationFee(_)))
            .collect();

        assert_eq!(
            created,
            vec![AccountDiff::ZkappAccountCreationFee(
                ZkappAccountCreationFee {
                    state_hash,
                    amount: MAINNET_ACCOUNT_CREATION_FEE,
                    public_key: "B62qnVLedrzTUMZME91WKbNw3qJ3hw7cc5PeK6RR3vH7RTFTsVbiBj4".into(),
                    token: TokenAddress::new("xosVXFFDvDiKvHSDAaHvrTSRtoa5Graf2J7LM5Smb4GNTrT2Hn")
                        .unwrap(),
                },
            )]
        );

        Ok(())
    }
}
