use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    command::{Command, UserCommandWithStatus, UserCommandWithStatusT},
    constants::ZKAPP_STATE_FIELD_ELEMENTS_NUM,
    ledger::{
        account::{Permissions, Timing},
        coinbase::Coinbase,
        nonce::Nonce,
        token::{symbol::TokenSymbol, TokenAddress},
        Amount, PublicKey,
    },
    mina_blocks::v2::{
        self,
        protocol_state::SupplyAdjustmentSign,
        staged_ledger_diff::{Elt, UpdateKind},
        ActionState, AppState, VerificationKey, ZkappUri,
    },
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
    pub amount: Amount,
    pub token: TokenAddress,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct DelegationDiff {
    pub nonce: Nonce,
    pub delegator: PublicKey,
    pub delegate: PublicKey,
}

/// Aggregated zkapp diff
/// Zkapps can:
/// - make token payments
/// - change app state elements
/// - change delegate
/// - change verification key
/// - change permissions
/// - change zkapp uri
/// - change token symbol
/// - change timing
/// - change voting for
/// - change actions
/// - change events
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappDiff {
    pub nonce: Nonce,
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub increment_nonce: bool,
    pub payment_diffs: Vec<PaymentDiff>,
    pub app_state_diff: [Option<AppState>; ZKAPP_STATE_FIELD_ELEMENTS_NUM],
    pub delegate: Option<PublicKey>,
    pub verification_key: Option<VerificationKey>,
    pub permissions: Option<Permissions>,
    pub zkapp_uri: Option<ZkappUri>,
    pub token_symbol: Option<TokenSymbol>,
    pub timing: Option<Timing>,
    pub voting_for: Option<BlockHash>,
    pub actions: Vec<ActionState>,
    pub events: Vec<EventState>,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappStateDiff {
    pub nonce: Option<Nonce>,
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub diffs: [Option<AppState>; ZKAPP_STATE_FIELD_ELEMENTS_NUM],
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappVerificationKeyDiff {
    pub nonce: Option<Nonce>,
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub verification_key: VerificationKey,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappPermissionsDiff {
    pub nonce: Option<Nonce>,
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub permissions: Permissions,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappUriDiff {
    pub nonce: Option<Nonce>,
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub zkapp_uri: ZkappUri,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappTokenSymbolDiff {
    pub nonce: Option<Nonce>,
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub token_symbol: TokenSymbol,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappTimingDiff {
    pub nonce: Option<Nonce>,
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub timing: Timing,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappVotingForDiff {
    pub nonce: Option<Nonce>,
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub voting_for: BlockHash,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappActionsDiff {
    pub nonce: Option<Nonce>,
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub actions: Vec<ActionState>,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappEventsDiff {
    pub nonce: Option<Nonce>,
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub events: Vec<EventState>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct EventState(pub String);

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

impl AccountDiff {
    pub fn token_address(&self) -> TokenAddress {
        if let &Self::Payment(PaymentDiff { ref token, .. }) = self {
            return token.to_owned();
        }

        TokenAddress::default()
    }

    pub fn from_command(command: Command) -> Vec<Vec<Self>> {
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
            Command::Zkapp(zkapp) => zkapp
                .account_updates
                .iter()
                .map(|update| {
                    let fee_payer = zkapp.fee_payer.body.public_key.to_owned();
                    let nonce = zkapp.fee_payer.body.nonce.into();

                    let mut diffs = vec![];
                    diffs.push((fee_payer.to_owned(), nonce, &update.elt).into());

                    for call in update.elt.calls.iter() {
                        diffs.push((fee_payer.to_owned(), nonce, call.elt.as_ref()).into());
                    }

                    diffs
                })
                .collect(),
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
        }
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
                if *total_fee > 0 {
                    res.push(vec![
                        AccountDiff::FeeTransfer(PaymentDiff {
                            public_key: prover.clone(),
                            amount: (*total_fee).into(),
                            update_type: UpdateType::Credit,
                            token: TokenAddress::default(), // always MINA
                        }),
                        AccountDiff::FeeTransfer(PaymentDiff {
                            public_key: precomputed_block.coinbase_receiver(),
                            amount: (*total_fee).into(),
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
            Zkapp(_)
            | ZkappStateDiff(_)
            | ZkappPermissionsDiff(_)
            | ZkappVerificationKeyDiff(_)
            | ZkappUriDiff(_)
            | ZkappTokenSymbolDiff(_)
            | ZkappTimingDiff(_)
            | ZkappVotingForDiff(_)
            | ZkappActionsDiff(_)
            | ZkappEventsDiff(_) => {
                unreachable!("zkapp commands do not have an amount")
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
                    nonce,
                    token: token.to_owned(),
                    public_key: sender.into(),
                    payment_diffs: vec![
                        PaymentDiff {
                            update_type: UpdateType::Credit,
                            public_key: receiver.into(),
                            amount: amount.into(),
                            token: token.to_owned(),
                        },
                        PaymentDiff {
                            update_type: UpdateType::Debit(Some(nonce)),
                            public_key: sender.into(),
                            amount: amount.into(),
                            token,
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

        let nonce = self.increment_nonce.then_some(self.nonce);

        // app state
        Self::expand_app_state_diff(
            &mut account_diffs,
            self.token.to_owned(),
            self.public_key.to_owned(),
            nonce,
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
            self.token.to_owned(),
            self.public_key.to_owned(),
            nonce,
            self.verification_key,
        );

        // permissions
        Self::expand_permissions_diff(
            &mut account_diffs,
            self.token.to_owned(),
            self.public_key.to_owned(),
            nonce,
            self.permissions,
        );

        // zkapp uri
        Self::expand_uri_diff(
            &mut account_diffs,
            self.token.to_owned(),
            self.public_key.to_owned(),
            nonce,
            self.zkapp_uri,
        );

        // token symbol
        Self::expand_symbol_diff(
            &mut account_diffs,
            self.token.to_owned(),
            self.public_key.to_owned(),
            nonce,
            self.token_symbol,
        );

        // timing
        Self::expand_timing_diff(
            &mut account_diffs,
            self.token.to_owned(),
            self.public_key.to_owned(),
            nonce,
            self.timing,
        );

        // voting for
        Self::expand_voting_for_diff(
            &mut account_diffs,
            self.token.to_owned(),
            self.public_key.to_owned(),
            nonce,
            self.voting_for,
        );

        // actions
        Self::expand_actions_diff(
            &mut account_diffs,
            self.token.to_owned(),
            self.public_key.to_owned(),
            nonce,
            self.actions,
        );

        // events
        Self::expand_events_diff(
            &mut account_diffs,
            self.token.to_owned(),
            self.public_key.to_owned(),
            nonce,
            self.events,
        );

        account_diffs
    }

    fn expand_payment_diff(account_diffs: &mut Vec<AccountDiff>, payment_diff: PaymentDiff) {
        account_diffs.push(AccountDiff::Payment(payment_diff));
    }

    fn expand_app_state_diff(
        account_diffs: &mut Vec<AccountDiff>,
        token: TokenAddress,
        pk: PublicKey,
        nonce: Option<Nonce>,
        app_state_diff: [Option<AppState>; ZKAPP_STATE_FIELD_ELEMENTS_NUM],
    ) {
        if !app_state_diff.iter().all(|state| state.is_none()) {
            account_diffs.push(AccountDiff::ZkappStateDiff(ZkappStateDiff {
                nonce,
                token,
                public_key: pk,
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
        token: TokenAddress,
        pk: PublicKey,
        nonce: Option<Nonce>,
        verification_key: Option<VerificationKey>,
    ) {
        if let Some(verification_key) = verification_key {
            account_diffs.push(AccountDiff::ZkappVerificationKeyDiff(
                ZkappVerificationKeyDiff {
                    nonce,
                    token,
                    public_key: pk,
                    verification_key,
                },
            ));
        }
    }

    fn expand_permissions_diff(
        account_diffs: &mut Vec<AccountDiff>,
        token: TokenAddress,
        pk: PublicKey,
        nonce: Option<Nonce>,
        permissions: Option<Permissions>,
    ) {
        if let Some(permissions) = permissions {
            account_diffs.push(AccountDiff::ZkappPermissionsDiff(ZkappPermissionsDiff {
                nonce,
                token,
                public_key: pk,
                permissions,
            }));
        }
    }

    fn expand_uri_diff(
        account_diffs: &mut Vec<AccountDiff>,
        token: TokenAddress,
        pk: PublicKey,
        nonce: Option<Nonce>,
        zkapp_uri: Option<ZkappUri>,
    ) {
        if let Some(zkapp_uri) = zkapp_uri {
            account_diffs.push(AccountDiff::ZkappUriDiff(ZkappUriDiff {
                nonce,
                token,
                public_key: pk,
                zkapp_uri,
            }));
        }
    }

    fn expand_symbol_diff(
        account_diffs: &mut Vec<AccountDiff>,
        token: TokenAddress,
        pk: PublicKey,
        nonce: Option<Nonce>,
        token_symbol: Option<TokenSymbol>,
    ) {
        if let Some(token_symbol) = token_symbol {
            account_diffs.push(AccountDiff::ZkappTokenSymbolDiff(ZkappTokenSymbolDiff {
                nonce,
                token,
                public_key: pk,
                token_symbol,
            }));
        }
    }

    fn expand_timing_diff(
        account_diffs: &mut Vec<AccountDiff>,
        token: TokenAddress,
        pk: PublicKey,
        nonce: Option<Nonce>,
        timing: Option<Timing>,
    ) {
        if let Some(timing) = timing {
            account_diffs.push(AccountDiff::ZkappTimingDiff(ZkappTimingDiff {
                nonce,
                token,
                public_key: pk,
                timing,
            }));
        }
    }

    fn expand_voting_for_diff(
        account_diffs: &mut Vec<AccountDiff>,
        token: TokenAddress,
        pk: PublicKey,
        nonce: Option<Nonce>,
        voting_for: Option<BlockHash>,
    ) {
        if let Some(voting_for) = voting_for {
            account_diffs.push(AccountDiff::ZkappVotingForDiff(ZkappVotingForDiff {
                nonce,
                token,
                public_key: pk,
                voting_for,
            }));
        }
    }

    fn expand_actions_diff(
        account_diffs: &mut Vec<AccountDiff>,
        token: TokenAddress,
        pk: PublicKey,
        nonce: Option<Nonce>,
        actions: Vec<ActionState>,
    ) {
        if !actions.is_empty() {
            account_diffs.push(AccountDiff::ZkappActionsDiff(ZkappActionsDiff {
                nonce,
                token,
                public_key: pk,
                actions,
            }));
        }
    }

    fn expand_events_diff(
        account_diffs: &mut Vec<AccountDiff>,
        token: TokenAddress,
        pk: PublicKey,
        nonce: Option<Nonce>,
        events: Vec<EventState>,
    ) {
        if !events.is_empty() {
            account_diffs.push(AccountDiff::ZkappEventsDiff(ZkappEventsDiff {
                nonce,
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
            Zkapp(diff) => diff.payment_diffs,
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
            | ZkappEventsDiff(_) => vec![],
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

// conversions

impl From<SupplyAdjustmentSign> for UpdateType {
    fn from(value: SupplyAdjustmentSign) -> Self {
        match value {
            SupplyAdjustmentSign::Neg => Self::Debit(None),
            SupplyAdjustmentSign::Pos => Self::Credit,
        }
    }
}

// zkapp account diffs
impl From<(PublicKey, Nonce, &Elt)> for AccountDiff {
    fn from(value: (PublicKey, Nonce, &Elt)) -> Self {
        // receiver
        let public_key = value.2.account_update.body.public_key.to_owned();

        let mut payment_diffs = vec![];
        let token = value.2.account_update.body.token_id.to_owned();

        // token payments
        let amount = value.2.account_update.body.balance_change.magnitude.into();
        let mut update_type: UpdateType = value
            .2
            .account_update
            .body
            .balance_change
            .sgn
            .0
            .to_owned()
            .into();

        if matches!(update_type, UpdateType::Debit(_)) {
            update_type = UpdateType::Debit(Some(value.1 + 1))
        }

        // pay creation fee of receiver zkapp
        if value.2.account_update.body.implicit_account_creation_fee {
            payment_diffs.push(PaymentDiff {
                public_key: value.0.to_owned(),
                update_type: UpdateType::Debit(Some(value.1 + 1)),
                token: token.to_owned(),
                amount,
            })
        }

        // increment nonce of receiver
        let increment_nonce = value.2.account_update.body.increment_nonce;
        if increment_nonce {
            payment_diffs.push(PaymentDiff {
                public_key: public_key.to_owned(),
                update_type: UpdateType::Debit(Some(value.1 + 1)),
                token: token.to_owned(),
                amount: 0.into(),
            });
        };

        // only push non-zero payments
        if amount.0 != 0 {
            payment_diffs.push(PaymentDiff {
                public_key: public_key.to_owned(),
                token: token.to_owned(),
                update_type,
                amount,
            });
        }

        // delegation change
        let delegate = value
            .2
            .account_update
            .body
            .update
            .delegate
            .to_owned()
            .into();

        // verification key change
        let verification_key = value
            .2
            .account_update
            .body
            .update
            .verification_key
            .to_owned()
            .into();

        // permissions change
        let permissions = {
            let permissions: Option<v2::Permissions> = value
                .2
                .account_update
                .body
                .update
                .permissions
                .to_owned()
                .into();
            permissions.map(Into::into)
        };

        // zkapp uri change
        let zkapp_uri = value
            .2
            .account_update
            .body
            .update
            .zkapp_uri
            .to_owned()
            .into();

        // token symbol change
        let token_symbol = value
            .2
            .account_update
            .body
            .update
            .token_symbol
            .to_owned()
            .into();

        // account timing change
        let timing = {
            let timing: Option<v2::Timing> =
                value.2.account_update.body.update.timing.to_owned().into();
            timing.map(Into::into)
        };

        // account `voting_for` change
        let voting_for = value
            .2
            .account_update
            .body
            .update
            .voting_for
            .to_owned()
            .into();

        // update actions
        let actions = if let Some(actions) = value.2.account_update.body.actions.first() {
            actions.0.iter().cloned().map(ActionState).collect()
        } else {
            vec![]
        };

        // update events
        let events = if let Some(events) = value.2.account_update.body.events.first() {
            events.0.iter().cloned().map(EventState).collect()
        } else {
            vec![]
        };

        Self::Zkapp(Box::new(ZkappDiff {
            token,
            public_key,
            nonce: value.1 + 1,
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
            app_state_diff: value.2.account_update.body.update.app_state.to_owned().map(
                |update_kind| match update_kind {
                    UpdateKind::Keep(_) => None,
                    UpdateKind::Set((_, state)) => Some(AppState(state)),
                },
            ),
        }))
    }
}

// debug/display

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
    use super::*;
    use crate::{
        block::precomputed::{PcbVersion, PrecomputedBlock},
        command::{Command, Delegation, Payment},
        constants::MAINNET_COINBASE_REWARD,
        ledger::{
            account::Permission,
            coinbase::{Coinbase, CoinbaseFeeTransfer, CoinbaseKind},
            diff::LedgerDiff,
            nonce::Nonce,
            token::TokenAddress,
            Amount, PublicKey,
        },
    };
    use std::path::PathBuf;
    use v2::{VerificationKeyData, VerificationKeyHash};

    #[test]
    fn test_amount() {
        let credit_amount = Amount(1000);
        let debit_amount = Amount(500);

        // Test Credit for PaymentDiff
        let payment_diff_credit = AccountDiff::Payment(PaymentDiff {
            public_key: PublicKey::new("B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG"),
            amount: credit_amount,
            update_type: UpdateType::Credit,
            token: TokenAddress::default(),
        });
        assert_eq!(payment_diff_credit.amount(), 1000);

        // Test Debit for PaymentDiff
        let payment_diff_debit = AccountDiff::Payment(PaymentDiff {
            public_key: PublicKey::new("B62qqmveaSLtpcfNeaF9KsEvLyjsoKvnfaHy4LHyApihPVzR3qDNNEG"),
            amount: debit_amount,
            update_type: UpdateType::Debit(Some(Nonce(1))),
            token: TokenAddress::default(),
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
            update_type: UpdateType::Credit,
            token: TokenAddress::default(),
        });
        assert_eq!(fee_transfer_diff_credit.amount(), 1000);

        // Test Debit for FeeTransferViaCoinbase PaymentDiff
        let fee_transfer_via_coinbase_diff_debit =
            AccountDiff::FeeTransferViaCoinbase(PaymentDiff {
                public_key: PublicKey::new(
                    "B62qkMUJyt7LmPnfu8in6qshaQSvTgLgNjx6h7YySRJ28wJegJ82n6u",
                ),
                amount: debit_amount,
                update_type: UpdateType::Debit(None),
                token: TokenAddress::default(),
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
                update_type: UpdateType::Credit,
                token: TokenAddress::default(),
            }),
            AccountDiff::Payment(PaymentDiff {
                public_key: source_public_key,
                amount: Amount(536900000000),
                update_type: UpdateType::Debit(Some(nonce + 1)),
                token: TokenAddress::default(),
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
            update_type: UpdateType::Debit(Some(nonce)),
            token: TokenAddress::default(),
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

    #[test]
    fn zkapp_account_diff() -> anyhow::Result<()> {
        let path = PathBuf::from("./tests/data/misc_blocks/mainnet-359630-3NLjRmTyUzeA7meRAT3Yjqxzfe95GKBgkLPD2iLeVE5RMCFcw8eL.json");
        let pcb = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;

        // all ledger diffs
        let diffs = LedgerDiff::from_precomputed_unexpanded(&pcb);

        // filter out non-zkapp account diffs
        let zkapp_diffs = diffs
            .account_diffs
            .into_iter()
            .filter_map(|diffs| {
                let diffs = diffs
                    .into_iter()
                    .filter_map(|diff| match diff {
                        AccountDiff::Zkapp(_) => Some(diff),
                        _ => None,
                    })
                    .collect::<Vec<_>>();

                // throw away non-zkapp account diffs
                if diffs.is_empty() {
                    None
                } else {
                    Some(diffs)
                }
            })
            .collect::<Vec<_>>();

        // expected unexpanded zkapp account diffs
        let expect = vec![
            vec![
                AccountDiff::Zkapp(Box::new(ZkappDiff {
                    nonce: 185.into(),
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    payment_diffs: vec![PaymentDiff {
                        public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5"
                            .into(),
                        update_type: UpdateType::Debit(Some(185.into())),
                        amount: 2000000000.into(),
                        token: TokenAddress::default(),
                    }],
                    ..Default::default()
                })),
                AccountDiff::Zkapp(Box::new(ZkappDiff {
                    nonce: 185.into(),
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    payment_diffs: vec![PaymentDiff {
                        public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5"
                            .into(),
                        update_type: UpdateType::Credit,
                        amount: 2000000000.into(),
                        token: TokenAddress::default(),
                    }],
                    ..Default::default()
                })),
            ],
            vec![
                AccountDiff::Zkapp(Box::new(ZkappDiff {
                    nonce: 186.into(),
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    payment_diffs: vec![PaymentDiff {
                        public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5"
                            .into(),
                        update_type: UpdateType::Debit(Some(186.into())),
                        amount: 2000000000.into(),
                        token: TokenAddress::default(),
                    }],
                    ..Default::default()
                })),
                AccountDiff::Zkapp(Box::new(ZkappDiff {
                    nonce: 186.into(),
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    payment_diffs: vec![PaymentDiff {
                        public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5"
                            .into(),
                        update_type: UpdateType::Credit,
                        amount: 2000000000.into(),
                        token: TokenAddress::default(),
                    }],
                    ..Default::default()
                })),
            ],
            vec![
                AccountDiff::Zkapp(Box::new(ZkappDiff {
                    nonce: 187.into(),
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    payment_diffs: vec![PaymentDiff {
                        public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5"
                            .into(),
                        update_type: UpdateType::Debit(Some(187.into())),
                        amount: 2000000000.into(),
                        token: TokenAddress::default(),
                    }],
                    ..Default::default()
                })),
                AccountDiff::Zkapp(Box::new(ZkappDiff {
                    nonce: 187.into(),
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    payment_diffs: vec![PaymentDiff {
                        public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5"
                            .into(),
                        update_type: UpdateType::Credit,
                        amount: 2000000000.into(),
                        token: TokenAddress::default(),
                    }],
                    ..Default::default()
                })),
            ],
            vec![AccountDiff::Zkapp(Box::new(ZkappDiff {
                nonce: 5.into(),
                public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".into(),
                payment_diffs: vec![
                    PaymentDiff {
                        public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
                            .into(),
                        update_type: UpdateType::Debit(Some(5.into())),
                        amount: 0.into(),
                        token: TokenAddress::default(),
                    },
                ],
                verification_key: Some(VerificationKey {
                    data: VerificationKeyData("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".to_string()),
                    hash: VerificationKeyHash("0x1C9320E5FD23AF1F8D8B1145484181C3E6B0F1C8C24FE4BDFFEF4281A61C3EBC".to_string())
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
                zkapp_uri: Some(ZkappUri("https://minainu.com".to_string())),
                token_symbol: Some(TokenSymbol("MINU".to_string())),
                increment_nonce: true,
                ..Default::default()
            }))],
        ];

        assert_eq!(zkapp_diffs, expect);

        // expected expanded zkapp diffs
        let expect: Vec<Vec<_>> = vec![
            vec![
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
                AccountDiff::Payment(PaymentDiff {
                    public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".into(),
                    update_type: UpdateType::Debit(Some(5.into())),
                    amount: 0.into(),
                    token: TokenAddress::default(),
                }),
                AccountDiff::ZkappVerificationKeyDiff(ZkappVerificationKeyDiff {
                    nonce: Some(Nonce(5)),
                    token: TokenAddress::default(),
                    public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".into(),
                    verification_key: VerificationKey {
                        data: VerificationKeyData("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".to_string()),
                        hash: VerificationKeyHash("0x1C9320E5FD23AF1F8D8B1145484181C3E6B0F1C8C24FE4BDFFEF4281A61C3EBC".to_string())
                    }
                }),
                AccountDiff::ZkappPermissionsDiff(ZkappPermissionsDiff {
                    nonce: Some(Nonce(5)),
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
                    nonce: Some(Nonce(5)),
                    token: TokenAddress::default(),
                    public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".into(),
                    zkapp_uri: ZkappUri("https://minainu.com".to_string())
                }),
                AccountDiff::ZkappTokenSymbolDiff(ZkappTokenSymbolDiff {
                    nonce: Some(Nonce(5)),
                    token: TokenAddress::default(),
                    public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".into(),
                    token_symbol: TokenSymbol("MINU".to_string())
                }),
            ],
        ];

        assert_eq!(AccountDiff::expand(zkapp_diffs), expect);
        Ok(())
    }
}
