//! Account diff representation

pub mod zkapp;

use crate::{
    base::nonce::Nonce,
    block::precomputed::PrecomputedBlock,
    command::{Command, CommandWithStateHash, UserCommandWithStatus, UserCommandWithStatusT},
    constants::MINA_TOKEN_ADDRESS,
    ledger::{
        coinbase::Coinbase,
        token::{account::TokenAccount, TokenAddress},
        Amount, PublicKey,
    },
    mina_blocks::v2::{
        self,
        protocol_state::SupplyAdjustmentSign,
        staged_ledger_diff::{
            Authorization, Call, Elt, Precondition, UpdateKind, ZkappCommandData,
        },
    },
    snark_work::SnarkWorkSummary,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zkapp::{
    ZkappActionsDiff, ZkappDiff, ZkappEventsDiff, ZkappFeePayerNonceDiff, ZkappIncrementNonceDiff,
    ZkappPaymentDiff, ZkappPermissionsDiff, ZkappProvedStateDiff, ZkappStateDiff, ZkappTimingDiff,
    ZkappTokenSymbolDiff, ZkappUriDiff, ZkappVerificationKeyDiff, ZkappVotingForDiff,
};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub enum AccountDiff {
    // non-zkapp
    Payment(PaymentDiff),
    Delegation(DelegationDiff),
    Coinbase(CoinbaseDiff),
    FeeTransfer(PaymentDiff),
    /// Overrides the fee transfer for SNARK work
    FeeTransferViaCoinbase(PaymentDiff),
    /// Updates the nonce for a failed txn
    FailedTransactionNonce(FailedTransactionNonceDiff),

    // zkapp
    Zkapp(Box<ZkappDiff>),
    ZkappPayment(ZkappPaymentDiff),
    ZkappState(ZkappStateDiff),
    ZkappPermissions(ZkappPermissionsDiff),
    ZkappVerificationKey(ZkappVerificationKeyDiff),
    ZkappProvedState(ZkappProvedStateDiff),
    ZkappUri(ZkappUriDiff),
    ZkappTokenSymbol(ZkappTokenSymbolDiff),
    ZkappTiming(ZkappTimingDiff),
    ZkappVotingFor(ZkappVotingForDiff),
    ZkappActions(ZkappActionsDiff),
    ZkappEvents(ZkappEventsDiff),
    ZkappIncrementNonce(ZkappIncrementNonceDiff),
    ZkappFeePayerNonce(ZkappFeePayerNonceDiff),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub enum UnapplyAccountDiff {
    // non-zkapp
    Payment(PaymentDiff),
    Delegation(DelegationDiff),
    Coinbase(CoinbaseDiff),
    FeeTransfer(PaymentDiff),
    /// Overrides the fee transfer for SNARK work
    FeeTransferViaCoinbase(PaymentDiff),
    /// Updates the nonce for a failed txn
    FailedTransactionNonce(FailedTransactionNonceDiff),

    // zkapp
    Zkapp(Box<ZkappDiff>),
    ZkappPayment(ZkappPaymentDiff),
    ZkappState(ZkappStateDiff),
    ZkappPermissions(ZkappPermissionsDiff),
    ZkappVerificationKey(ZkappVerificationKeyDiff),
    ZkappProvedState(ZkappProvedStateDiff),
    ZkappUri(ZkappUriDiff),
    ZkappTokenSymbol(ZkappTokenSymbolDiff),
    ZkappTiming(ZkappTimingDiff),
    ZkappVotingFor(ZkappVotingForDiff),
    ZkappActions(ZkappActionsDiff),
    ZkappEvents(ZkappEventsDiff),
    ZkappIncrementNonce(ZkappIncrementNonceDiff),
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
    pub amount: Amount,
    pub update_type: UpdateType,
    pub public_key: PublicKey,
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
            Self::ZkappPayment(diff) => diff.public_key(),
            Self::ZkappState(diff) => diff.public_key.clone(),
            Self::ZkappPermissions(diff) => diff.public_key.clone(),
            Self::ZkappVerificationKey(diff) => diff.public_key.clone(),
            Self::ZkappProvedState(diff) => diff.public_key.clone(),
            Self::ZkappUri(diff) => diff.public_key.clone(),
            Self::ZkappTokenSymbol(diff) => diff.public_key.clone(),
            Self::ZkappTiming(diff) => diff.public_key.clone(),
            Self::ZkappVotingFor(diff) => diff.public_key.clone(),
            Self::ZkappActions(diff) => diff.public_key.clone(),
            Self::ZkappEvents(diff) => diff.public_key.clone(),
            Self::ZkappIncrementNonce(diff) => diff.public_key.clone(),
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
            Self::ZkappPayment(diff) => diff.token(),
            Self::ZkappState(diff) => diff.token.clone(),
            Self::ZkappPermissions(diff) => diff.token.clone(),
            Self::ZkappVerificationKey(diff) => diff.token.clone(),
            Self::ZkappProvedState(diff) => diff.token.clone(),
            Self::ZkappUri(diff) => diff.token.clone(),
            Self::ZkappTokenSymbol(diff) => diff.token.clone(),
            Self::ZkappTiming(diff) => diff.token.clone(),
            Self::ZkappVotingFor(diff) => diff.token.clone(),
            Self::ZkappActions(diff) => diff.token.clone(),
            Self::ZkappEvents(diff) => diff.token.clone(),
            Self::ZkappIncrementNonce(diff) => diff.token.clone(),
        }
    }
}

impl AccountDiff {
    pub fn from_command(command: CommandWithStateHash, global_slot: u32) -> Vec<Vec<Self>> {
        let CommandWithStateHash {
            command,
            state_hash: _,
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
            Command::Zkapp(zkapp) => AccountDiff::from_zkapp(&zkapp, global_slot),
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
            Self::ZkappPayment(diff) => ZkappPayment(diff),
            Self::ZkappState(diff) => ZkappState(diff),
            Self::ZkappPermissions(diff) => ZkappPermissions(diff),
            Self::ZkappVerificationKey(diff) => ZkappVerificationKey(diff),
            Self::ZkappProvedState(diff) => ZkappProvedState(diff),
            Self::ZkappUri(diff) => ZkappUri(diff),
            Self::ZkappTokenSymbol(diff) => ZkappTokenSymbol(diff),
            Self::ZkappTiming(diff) => ZkappTiming(diff),
            Self::ZkappVotingFor(diff) => ZkappVotingFor(diff),
            Self::ZkappActions(diff) => ZkappActions(diff),
            Self::ZkappEvents(diff) => ZkappEvents(diff),
            Self::ZkappIncrementNonce(diff) => ZkappIncrementNonce(diff),
            Self::ZkappFeePayerNonce(diff) => ZkappFeePayerNonce(diff),
        }
    }

    pub fn from_coinbase(coinbase: Coinbase) -> Vec<Vec<Self>> {
        let mut res = if !coinbase.is_applied() {
            vec![]
        } else {
            vec![vec![Self::Coinbase(CoinbaseDiff {
                public_key: coinbase.receiver.clone(),
                amount: coinbase.amount().into(),
            })]]
        };

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
            ZkappPayment(ZkappPaymentDiff::Payment(diff)) => diff.balance_change(),
            Zkapp(_)
            | ZkappPayment(_)
            | ZkappState(_)
            | ZkappPermissions(_)
            | ZkappVerificationKey(_)
            | ZkappProvedState(_)
            | ZkappUri(_)
            | ZkappTokenSymbol(_)
            | ZkappTiming(_)
            | ZkappVotingFor(_)
            | ZkappActions(_)
            | ZkappEvents(_)
            | ZkappIncrementNonce(_)
            | ZkappFeePayerNonce(_) => {
                unreachable!("zkapp commands do not have an amount")
            }
        }
    }

    pub fn is_zkapp_diff(&self) -> bool {
        !matches!(
            self,
            AccountDiff::Coinbase(_)
                | AccountDiff::Payment(_)
                | AccountDiff::Delegation(_)
                | AccountDiff::FeeTransfer(_)
                | AccountDiff::FeeTransferViaCoinbase(_)
                | AccountDiff::FailedTransactionNonce(_)
        )
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
                    nonce: Some(nonce),
                    token: token.to_owned(),
                    public_key: sender.into(),
                    payment_diffs: vec![
                        ZkappPaymentDiff::Payment(PaymentDiff {
                            update_type: UpdateType::Credit,
                            public_key: receiver.into(),
                            amount: amount.into(),
                            token: token.to_owned(),
                        }),
                        ZkappPaymentDiff::Payment(PaymentDiff {
                            update_type: UpdateType::Debit(None),
                            public_key: sender.into(),
                            amount: amount.into(),
                            token,
                        }),
                    ],
                    ..Default::default()
                }))]]
            }
        }
    }
}

impl PaymentDiff {
    pub fn balance_change(&self) -> i64 {
        let Self {
            amount,
            update_type,
            ..
        } = self;
        match update_type {
            UpdateType::Credit => amount.0 as i64,
            UpdateType::Debit(_) => -(amount.0 as i64),
        }
    }

    pub fn from_account_diff(diff: AccountDiff) -> Vec<Self> {
        match diff {
            AccountDiff::Zkapp(diff) => diff
                .payment_diffs
                .into_iter()
                .filter_map(|diff| match diff {
                    ZkappPaymentDiff::Payment(payment) => Some(payment),
                    ZkappPaymentDiff::IncrementNonce(_) => None,
                })
                .collect(),
            AccountDiff::Payment(diff)
            | AccountDiff::FeeTransfer(diff)
            | AccountDiff::FeeTransferViaCoinbase(diff)
            | AccountDiff::ZkappPayment(ZkappPaymentDiff::Payment(diff)) => {
                vec![diff]
            }
            AccountDiff::Coinbase(cb_diff) => vec![Self {
                update_type: UpdateType::Credit,
                public_key: cb_diff.public_key,
                amount: cb_diff.amount,
                token: TokenAddress::default(), // always MINA
            }],
            AccountDiff::Delegation(_)
            | AccountDiff::FailedTransactionNonce(_)
            | AccountDiff::ZkappPayment(ZkappPaymentDiff::IncrementNonce(_))
            | AccountDiff::ZkappState(_)
            | AccountDiff::ZkappPermissions(_)
            | AccountDiff::ZkappVerificationKey(_)
            | AccountDiff::ZkappProvedState(_)
            | AccountDiff::ZkappUri(_)
            | AccountDiff::ZkappTokenSymbol(_)
            | AccountDiff::ZkappTiming(_)
            | AccountDiff::ZkappVotingFor(_)
            | AccountDiff::ZkappActions(_)
            | AccountDiff::ZkappEvents(_)
            | AccountDiff::ZkappIncrementNonce(_)
            | AccountDiff::ZkappFeePayerNonce(_) => vec![],
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
    fn from_zkapp(zkapp: &ZkappCommandData, global_slot: u32) -> Vec<Vec<Self>> {
        let nonce = zkapp.fee_payer.body.nonce;

        // fee payer nonce
        let mut diffs = vec![AccountDiff::ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
            public_key: zkapp.fee_payer.body.public_key.to_owned(),
            nonce: nonce + 1,
        })];

        for update in zkapp.account_updates.iter() {
            diffs.push(Self::from_zkapp_account_update(&update.elt, global_slot));

            recurse_calls(&mut diffs, update.elt.calls.iter(), global_slot);
        }

        vec![diffs]
    }

    fn from_zkapp_account_update(elt: &Elt, global_slot: u32) -> Self {
        let public_key = elt.account_update.body.public_key.to_owned();

        let mut payment_diffs = vec![];
        let token = elt.account_update.body.token_id.to_owned();

        // token payments
        let amount = elt.account_update.body.balance_change.magnitude.0.into();

        // pay creation fee of receiver zkapp
        if elt.account_update.body.implicit_account_creation_fee {
            payment_diffs.push(ZkappPaymentDiff::Payment(PaymentDiff {
                public_key: public_key.to_owned(),
                update_type: UpdateType::Debit(None),
                token: token.to_owned(),
                amount,
            }));
        }

        // increment nonce of updated account
        let increment_nonce = elt.account_update.body.increment_nonce;
        if increment_nonce {
            payment_diffs.push(ZkappPaymentDiff::IncrementNonce(ZkappIncrementNonceDiff {
                public_key: public_key.to_owned(),
                token: token.to_owned(),
            }));
        };

        payment_diffs.push(ZkappPaymentDiff::Payment(PaymentDiff {
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
        }));

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
            token,
            public_key,
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
            global_slot,
            ..Default::default()
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
        write!(f, "{} | {}", self.public_key, self.nonce)
    }
}

impl std::fmt::Display for AccountDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Payment(diff) => write!(f, "PAYMENT: {diff:?}"),
            Self::Delegation(diff) => write!(f, "DELEGATION: {diff:?}"),
            Self::Coinbase(diff) => write!(f, "COINBASE: {diff:?}"),
            Self::FeeTransfer(diff) => write!(f, "FEE_TRANSFER: {diff:?}"),
            Self::FeeTransferViaCoinbase(diff) => {
                write!(f, "FEE_TRANSFER_VIA_COINBASE: {diff:?}")
            }
            Self::FailedTransactionNonce(diff) => {
                write!(f, "FAILED_TXN: {diff:?}")
            }
            Self::Zkapp(diff) => write!(f, "ZKAPP: {diff:?}"),
            Self::ZkappPayment(ZkappPaymentDiff::Payment(diff)) => {
                write!(f, "{:<27}{diff:?}", "ZkappPayment:")
            }
            Self::ZkappPayment(ZkappPaymentDiff::IncrementNonce(diff)) => {
                write!(f, "{:<27}{diff:?}", "ZkappIncrementNonce:")
            }
            Self::ZkappState(diff) => write!(f, "ZKAPP_STATE: {diff:?}"),
            Self::ZkappPermissions(diff) => write!(f, "ZKAPP_PERMISSIONS: {diff:?}"),
            Self::ZkappVerificationKey(diff) => write!(f, "ZKAPP_VK: {diff:?}"),
            Self::ZkappProvedState(diff) => write!(f, "ZKAPP_PROVED_STATE: {diff:?}"),
            Self::ZkappUri(diff) => write!(f, "ZKAPP_URI: {diff:?}"),
            Self::ZkappTokenSymbol(diff) => {
                write!(f, "ZKAPP_TOKEN_SYMBOL: {diff:?}")
            }
            Self::ZkappTiming(diff) => write!(f, "ZKAPP_TIMING: {diff:?}"),
            Self::ZkappVotingFor(diff) => write!(f, "ZKAPP_VOTING_FOR: {diff:?}"),
            Self::ZkappActions(diff) => write!(f, "ZKAPP_ACTIONS: {diff:?}"),
            Self::ZkappEvents(diff) => write!(f, "ZKAPP_EVENTS: {diff:?}"),
            Self::ZkappIncrementNonce(diff) => {
                write!(f, "ZKAPP_INCREMENT_NONCE {}", diff.public_key)
            }
            Self::ZkappFeePayerNonce(diff) => write!(f, "ZKAPP_FEE_PAYER_NONCE: {diff:?}"),
        }
    }
}

impl std::fmt::Debug for AccountDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Payment(diff) => write!(f, "{:<27}{diff:?}", "Payment:"),
            Self::Delegation(diff) => write!(f, "{:<27}{diff:?}", "Delegation:"),
            Self::Coinbase(diff) => write!(f, "{:<27}{diff:?}", "Coinbase:"),
            Self::FeeTransfer(diff) => write!(f, "{:<27}{diff:?}", "Fee transfer:"),
            Self::FeeTransferViaCoinbase(diff) => {
                write!(f, "{:<27}{diff:?}", "Fee transfer via coinbase:")
            }
            Self::FailedTransactionNonce(diff) => {
                write!(f, "{:<27}{diff:?}", "Failed transaction:")
            }
            Self::Zkapp(diff) => write!(f, "{:<27}{diff:?}", "Zkapp:"),
            Self::ZkappPayment(ZkappPaymentDiff::Payment(diff)) => {
                write!(
                    f,
                    "{:<27}{diff:?}{}",
                    "ZkappPayment:",
                    if diff.token.0 == MINA_TOKEN_ADDRESS {
                        "".to_string()
                    } else {
                        format!(" | {}", diff.token.0)
                    }
                )
            }
            Self::ZkappPayment(ZkappPaymentDiff::IncrementNonce(diff)) => {
                write!(f, "{:<27}{diff:?}", "ZkappIncrementNonce:")
            }
            Self::ZkappState(diff) => write!(f, "{:<27}{diff:?}", "ZkappState:"),
            Self::ZkappPermissions(diff) => write!(f, "{:<27}: {diff:?}", "ZkappPermissions:"),
            Self::ZkappVerificationKey(diff) => write!(f, "{:<27}{diff:?}", "ZkappVK:"),
            Self::ZkappProvedState(diff) => write!(f, "{:<27}{diff:?}", "ZkappProvedState:"),
            Self::ZkappUri(diff) => write!(f, "{:<27}{diff:?}", "ZkappUri:"),
            Self::ZkappTokenSymbol(diff) => {
                write!(f, "{:<27}{diff:?}", "ZkappTokenSymbol:")
            }
            Self::ZkappTiming(diff) => write!(f, "{:<27}{diff:?}", "ZkappTiming:"),
            Self::ZkappVotingFor(diff) => write!(f, "{:<27}{diff:?}", "ZkappVotingFor:"),
            Self::ZkappActions(diff) => write!(f, "{:<27}{diff:?}", "ZkappActions:"),
            Self::ZkappEvents(diff) => write!(f, "{:<27}{diff:?}", "ZkappEvents:"),
            Self::ZkappIncrementNonce(diff) => {
                write!(f, "{:<27}{}", "ZkappIncrementNonce:", diff.public_key)
            }
            Self::ZkappFeePayerNonce(diff) => write!(f, "{:<27}{diff:?}", "ZkappFeePayerNonce:"),
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
    global_slot: u32,
) {
    for call in calls {
        diffs.push(AccountDiff::from_zkapp_account_update(
            call.elt.as_ref(),
            global_slot,
        ));

        recurse_calls(diffs, call.elt.calls.iter(), global_slot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        base::{nonce::Nonce, state_hash::StateHash},
        block::precomputed::{PcbVersion, PrecomputedBlock},
        command::{AccountUpdate, Command, Delegation, Payment},
        constants::MAINNET_COINBASE_REWARD,
        ledger::{
            coinbase::{Coinbase, CoinbaseFeeTransfer, CoinbaseKind},
            diff::LedgerDiff,
            token::TokenAddress,
            Amount, PublicKey,
        },
        mina_blocks::v2::staged_ledger_diff::{UserCommand, UserCommandData},
        store::Result,
    };
    use std::path::PathBuf;

    #[test]
    fn test_amount() -> Result<()> {
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
        let global_slot = 100;
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

        assert_eq!(
            AccountDiff::from_command(payment_command, global_slot),
            expected_result
        );
    }

    #[test]
    fn test_from_command_delegation() {
        let nonce = Nonce(42);
        let global_slot = 100;
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
            AccountDiff::from_command(delegation_command, global_slot),
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
    fn test_public_key_payment() -> Result<()> {
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
    fn test_snark_account_creation_deduction() -> Result<()> {
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
    fn genesis_v1() -> Result<()> {
        let path = PathBuf::from("./data/genesis_blocks/mainnet-1-3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ.json");
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V1)?;
        let diff = LedgerDiff::from_precomputed(&block);

        assert!(diff.account_diffs.is_empty(), "{:#?}", diff.account_diffs);
        Ok(())
    }

    #[test]
    fn genesis_v2() -> Result<()> {
        let path = PathBuf::from("./data/genesis_blocks/mainnet-359605-3NK4BpDSekaqsG6tx8Nse2zJchRft2JpnbvMiog55WCr5xJZaKeP.json");
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;
        let diff = LedgerDiff::from_precomputed(&block);

        assert!(diff.account_diffs.is_empty(), "{:#?}", diff.account_diffs);
        Ok(())
    }

    #[test]
    fn zkapp_account_updates() -> Result<()> {
        let path = PathBuf::from("./tests/data/misc_blocks/mainnet-368442-3NLTFUdvKixsbCqEbjWKskrjWuaSQpwTjoGNXWzK7eaUn4oHscbu.json");
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;

        let zkapps = block.zkapp_commands();
        let zkapp_cmd = zkapps.first().unwrap();

        if let UserCommandWithStatus::V2(box_cmd) = zkapp_cmd {
            if let UserCommand {
                data: (_, UserCommandData::ZkappCommandData(ZkappCommandData { .. })),
                ..
            } = box_cmd.as_ref()
            {
                let expect = vec![
                    AccountUpdate {
                        public_key: "B62qkikiZXisUGspBunSnKQn5FRaUPkLUBbxkBY64Xn6AnaSwgKab5h"
                            .into(),
                        token: TokenAddress::new(
                            "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf",
                        )
                        .unwrap(),
                        balance_change: -1000000000,
                        increment_nonce: false,
                    },
                    AccountUpdate {
                        public_key: "B62qjwDWxjf4LtJ4YWJQDdTNPqZ69ZyeCzbpAFKN7EoZzYig5ZRz8JE"
                            .into(),
                        token: TokenAddress::new(
                            "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf",
                        )
                        .unwrap(),
                        balance_change: 0,
                        increment_nonce: false,
                    },
                    AccountUpdate {
                        public_key: "B62qjwDWxjf4LtJ4YWJQDdTNPqZ69ZyeCzbpAFKN7EoZzYig5ZRz8JE"
                            .into(),
                        token: TokenAddress::new(
                            "xBxjFpJkbWpbGua7Lf36S1NLhffFoEChyP3pz6SYKnx7dFCTwg",
                        )
                        .unwrap(),
                        balance_change: -10000000000,
                        increment_nonce: false,
                    },
                    AccountUpdate {
                        public_key: "B62qnVgC5sXACSeAAYV7wjeLYFeC3XZ1PA2MBsuSUUsqiK96jfN9sba"
                            .into(),
                        token: TokenAddress::new(
                            "xBxjFpJkbWpbGua7Lf36S1NLhffFoEChyP3pz6SYKnx7dFCTwg",
                        )
                        .unwrap(),
                        balance_change: 10000000000,
                        increment_nonce: false,
                    },
                ];

                assert_eq!(zkapp_cmd.accounts_updated(), expect);
            }
        } else {
            panic!("Expected a zkapp command")
        }

        Ok(())
    }

    #[test]
    fn zkapp_account_diffs() -> Result<()> {
        let path = PathBuf::from("./tests/data/misc_blocks/mainnet-368442-3NLTFUdvKixsbCqEbjWKskrjWuaSQpwTjoGNXWzK7eaUn4oHscbu.json");
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;

        let state_hash = block.state_hash();
        let global_slot = block.global_slot_since_genesis();

        use AccountDiff::*;
        let zkapps = block.zkapp_commands();
        let zkapp_cmd = zkapps.first().unwrap();

        if let UserCommandWithStatus::V2(box_cmd) = zkapp_cmd {
            if let UserCommand {
                data: (_, UserCommandData::ZkappCommandData(zkapp)),
                ..
            } = box_cmd.as_ref()
            {
                let expect = vec![vec![
                    ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                        public_key: "B62qkikiZXisUGspBunSnKQn5FRaUPkLUBbxkBY64Xn6AnaSwgKab5h"
                            .into(),
                        nonce: 59.into(),
                    }),
                    ZkappPayment(ZkappPaymentDiff::Payment(PaymentDiff {
                        amount: 1000000000.into(),
                        update_type: UpdateType::Debit(None),
                        public_key: "B62qkikiZXisUGspBunSnKQn5FRaUPkLUBbxkBY64Xn6AnaSwgKab5h"
                            .into(),
                        token: TokenAddress::default(),
                    })),
                    ZkappPayment(ZkappPaymentDiff::Payment(PaymentDiff {
                        amount: 0.into(),
                        update_type: UpdateType::Credit,
                        public_key: "B62qjwDWxjf4LtJ4YWJQDdTNPqZ69ZyeCzbpAFKN7EoZzYig5ZRz8JE"
                            .into(),
                        token: TokenAddress::default(),
                    })),
                    ZkappEvents(ZkappEventsDiff {
                        token: TokenAddress::default(),
                        public_key: "B62qjwDWxjf4LtJ4YWJQDdTNPqZ69ZyeCzbpAFKN7EoZzYig5ZRz8JE"
                            .into(),
                        events: vec![
                            "0x0000000000000000000000000000000000000000000000000000000000000002"
                                .into(),
                            "0x01B2700CB8B5AB3EA1E6901ED662EDBD45F1FADEDFDA70A406F2E36F7A902F2C"
                                .into(),
                            "0x0000000000000000000000000000000000000000000000000000000000000001"
                                .into(),
                            "0x3D76374FA52F749B664DB992AF45C57F92535C1CDAED68867781673A7E278F78"
                                .into(),
                            "0x0000000000000000000000000000000000000000000000000000000000000001"
                                .into(),
                            "0x00000000000000000000000000000000000000000000000000000002540BE400"
                                .into(),
                        ],
                    }),
                    ZkappPayment(ZkappPaymentDiff::Payment(PaymentDiff {
                        amount: 10000000000.into(),
                        update_type: UpdateType::Debit(None),
                        public_key: "B62qjwDWxjf4LtJ4YWJQDdTNPqZ69ZyeCzbpAFKN7EoZzYig5ZRz8JE"
                            .into(),
                        token: TokenAddress::new(
                            "xBxjFpJkbWpbGua7Lf36S1NLhffFoEChyP3pz6SYKnx7dFCTwg",
                        )
                        .unwrap(),
                    })),
                    ZkappPayment(ZkappPaymentDiff::Payment(PaymentDiff {
                        amount: 10000000000.into(),
                        update_type: UpdateType::Credit,
                        public_key: "B62qnVgC5sXACSeAAYV7wjeLYFeC3XZ1PA2MBsuSUUsqiK96jfN9sba"
                            .into(),
                        token: TokenAddress::new(
                            "xBxjFpJkbWpbGua7Lf36S1NLhffFoEChyP3pz6SYKnx7dFCTwg",
                        )
                        .unwrap(),
                    })),
                ]];

                let diffs = AccountDiff::from_command(
                    CommandWithStateHash {
                        command: Command::Zkapp(zkapp.clone()),
                        state_hash,
                    },
                    global_slot,
                );

                assert_eq!(AccountDiff::expand(diffs), expect);
                return Ok(());
            }
        }

        panic!("Expected a zkapp command")
    }
}
