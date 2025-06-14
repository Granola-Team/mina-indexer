//! Zkapp account ledger diff representation

use super::{AccountDiff, DelegationDiff, PaymentDiff};
use crate::{
    base::nonce::Nonce,
    command::TxnHash,
    constants::ZKAPP_STATE_FIELD_ELEMENTS_NUM,
    ledger::{
        account::{Permissions, Timing, VotingFor},
        token::{TokenAddress, TokenSymbol},
        PublicKey,
    },
    mina_blocks::v2::{ActionState, AppState, VerificationKey, ZkappEvent, ZkappUri},
};
use serde::{Deserialize, Serialize};

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
    pub nonce: Option<Nonce>,
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub increment_nonce: bool,
    pub proved_state: bool,
    pub payment_diffs: Vec<ZkappPaymentDiff>,
    pub app_state_diff: [Option<AppState>; ZKAPP_STATE_FIELD_ELEMENTS_NUM],
    pub delegate: Option<PublicKey>,
    pub verification_key: Option<VerificationKey>,
    pub permissions: Option<Permissions>,
    pub zkapp_uri: Option<ZkappUri>,
    pub token_symbol: Option<TokenSymbol>,
    pub timing: Option<Timing>,
    pub voting_for: Option<VotingFor>,
    pub actions: Vec<ActionState>,
    pub events: Vec<ZkappEvent>,
    pub global_slot: u32,
    pub creation_fee_paid: bool,
    pub txn_hash: TxnHash,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub enum ZkappPaymentDiff {
    Payment {
        payment: PaymentDiff,
        creation_fee_paid: bool,
    },
    IncrementNonce(ZkappIncrementNonceDiff),
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappStateDiff {
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub diffs: [Option<AppState>; ZKAPP_STATE_FIELD_ELEMENTS_NUM],
    pub txn_hash: TxnHash,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappVerificationKeyDiff {
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub verification_key: VerificationKey,
    pub txn_hash: TxnHash,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappProvedStateDiff {
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub proved_state: bool,
    pub txn_hash: TxnHash,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappPermissionsDiff {
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub permissions: Permissions,
    pub txn_hash: TxnHash,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappUriDiff {
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub zkapp_uri: ZkappUri,
    pub txn_hash: TxnHash,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappTokenSymbolDiff {
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub token_symbol: TokenSymbol,
    pub txn_hash: TxnHash,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappTimingDiff {
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub timing: Timing,
    pub txn_hash: TxnHash,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappVotingForDiff {
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub voting_for: VotingFor,
    pub txn_hash: TxnHash,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappActionsDiff {
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub actions: Vec<ActionState>,
    pub global_slot: u32,
    pub txn_hash: TxnHash,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappEventsDiff {
    pub token: TokenAddress,
    pub public_key: PublicKey,
    pub events: Vec<ZkappEvent>,
    pub txn_hash: TxnHash,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappIncrementNonceDiff {
    pub public_key: PublicKey,
    pub token: TokenAddress,
    pub creation_fee_paid: bool,
    pub txn_hash: TxnHash,
}

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappFeePayerNonceDiff {
    pub public_key: PublicKey,
    pub nonce: Nonce,
    pub txn_hash: TxnHash,
}

///////////
// impls //
///////////

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
            self.token.to_owned(),
            self.public_key.to_owned(),
            self.app_state_diff,
            self.txn_hash.to_owned(),
        );

        // delegate
        Self::expand_delegate_diff(
            &mut account_diffs,
            self.public_key.to_owned(),
            self.nonce.unwrap_or_default(),
            self.delegate,
            self.txn_hash.to_owned(),
        );

        // verification key
        Self::expand_verification_key_diff(
            &mut account_diffs,
            self.token.to_owned(),
            self.public_key.to_owned(),
            self.verification_key,
            self.txn_hash.to_owned(),
        );

        Self::expand_proved_state_diff(
            &mut account_diffs,
            self.token.to_owned(),
            self.public_key.to_owned(),
            self.proved_state,
            self.txn_hash.to_owned(),
        );

        // permissions
        Self::expand_permissions_diff(
            &mut account_diffs,
            self.token.to_owned(),
            self.public_key.to_owned(),
            self.permissions,
            self.txn_hash.to_owned(),
        );

        // zkapp uri
        Self::expand_uri_diff(
            &mut account_diffs,
            self.token.to_owned(),
            self.public_key.to_owned(),
            self.zkapp_uri,
            self.txn_hash.to_owned(),
        );

        // token symbol
        Self::expand_symbol_diff(
            &mut account_diffs,
            self.token.to_owned(),
            self.public_key.to_owned(),
            self.token_symbol,
            self.txn_hash.to_owned(),
        );

        // timing
        Self::expand_timing_diff(
            &mut account_diffs,
            self.token.to_owned(),
            self.public_key.to_owned(),
            self.timing,
            self.txn_hash.to_owned(),
        );

        // voting for
        Self::expand_voting_for_diff(
            &mut account_diffs,
            self.token.to_owned(),
            self.public_key.to_owned(),
            self.voting_for,
            self.txn_hash.to_owned(),
        );

        // actions
        Self::expand_actions_diff(
            &mut account_diffs,
            self.token.to_owned(),
            self.public_key.to_owned(),
            self.actions,
            self.global_slot,
            self.txn_hash.to_owned(),
        );

        // events
        Self::expand_events_diff(
            &mut account_diffs,
            self.token.to_owned(),
            self.public_key.to_owned(),
            self.events,
            self.txn_hash.to_owned(),
        );

        account_diffs
    }

    fn expand_payment_diff(account_diffs: &mut Vec<AccountDiff>, diff: ZkappPaymentDiff) {
        use ZkappPaymentDiff::*;

        let acct_diff = match diff {
            IncrementNonce(diff) => AccountDiff::ZkappIncrementNonce(diff),
            Payment { .. } => AccountDiff::ZkappPayment(diff),
        };

        account_diffs.push(acct_diff)
    }

    fn expand_app_state_diff(
        account_diffs: &mut Vec<AccountDiff>,
        token: TokenAddress,
        pk: PublicKey,
        app_state_diff: [Option<AppState>; ZKAPP_STATE_FIELD_ELEMENTS_NUM],
        txn_hash: TxnHash,
    ) {
        if !app_state_diff.iter().all(|state| state.is_none()) {
            account_diffs.push(AccountDiff::ZkappState(ZkappStateDiff {
                token,
                public_key: pk,
                diffs: app_state_diff,
                txn_hash,
            }));
        }
    }

    fn expand_delegate_diff(
        account_diffs: &mut Vec<AccountDiff>,
        pk: PublicKey,
        nonce: Nonce,
        delegate: Option<PublicKey>,
        txn_hash: TxnHash,
    ) {
        if let Some(delegate) = delegate {
            account_diffs.push(AccountDiff::Delegation(DelegationDiff {
                nonce,
                delegator: pk,
                delegate,
                txn_hash,
            }));
        }
    }

    fn expand_verification_key_diff(
        account_diffs: &mut Vec<AccountDiff>,
        token: TokenAddress,
        pk: PublicKey,
        verification_key: Option<VerificationKey>,
        txn_hash: TxnHash,
    ) {
        if let Some(verification_key) = verification_key {
            account_diffs.push(AccountDiff::ZkappVerificationKey(
                ZkappVerificationKeyDiff {
                    token,
                    public_key: pk,
                    verification_key,
                    txn_hash,
                },
            ));
        }
    }

    fn expand_proved_state_diff(
        account_diffs: &mut Vec<AccountDiff>,
        token: TokenAddress,
        pk: PublicKey,
        proved_state: bool,
        txn_hash: TxnHash,
    ) {
        if proved_state {
            account_diffs.push(AccountDiff::ZkappProvedState(ZkappProvedStateDiff {
                token,
                public_key: pk,
                proved_state,
                txn_hash,
            }));
        }
    }

    fn expand_permissions_diff(
        account_diffs: &mut Vec<AccountDiff>,
        token: TokenAddress,
        pk: PublicKey,
        permissions: Option<Permissions>,
        txn_hash: TxnHash,
    ) {
        if let Some(permissions) = permissions {
            account_diffs.push(AccountDiff::ZkappPermissions(ZkappPermissionsDiff {
                token,
                public_key: pk,
                permissions,
                txn_hash,
            }));
        }
    }

    fn expand_uri_diff(
        account_diffs: &mut Vec<AccountDiff>,
        token: TokenAddress,
        pk: PublicKey,
        zkapp_uri: Option<ZkappUri>,
        txn_hash: TxnHash,
    ) {
        if let Some(zkapp_uri) = zkapp_uri {
            account_diffs.push(AccountDiff::ZkappUri(ZkappUriDiff {
                token,
                public_key: pk,
                zkapp_uri,
                txn_hash,
            }));
        }
    }

    fn expand_symbol_diff(
        account_diffs: &mut Vec<AccountDiff>,
        token: TokenAddress,
        pk: PublicKey,
        token_symbol: Option<TokenSymbol>,
        txn_hash: TxnHash,
    ) {
        if let Some(token_symbol) = token_symbol {
            account_diffs.push(AccountDiff::ZkappTokenSymbol(ZkappTokenSymbolDiff {
                token,
                public_key: pk,
                token_symbol,
                txn_hash,
            }));
        }
    }

    fn expand_timing_diff(
        account_diffs: &mut Vec<AccountDiff>,
        token: TokenAddress,
        pk: PublicKey,
        timing: Option<Timing>,
        txn_hash: TxnHash,
    ) {
        if let Some(timing) = timing {
            account_diffs.push(AccountDiff::ZkappTiming(ZkappTimingDiff {
                token,
                public_key: pk,
                timing,
                txn_hash,
            }));
        }
    }

    fn expand_voting_for_diff(
        account_diffs: &mut Vec<AccountDiff>,
        token: TokenAddress,
        pk: PublicKey,
        voting_for: Option<VotingFor>,
        txn_hash: TxnHash,
    ) {
        if let Some(voting_for) = voting_for {
            account_diffs.push(AccountDiff::ZkappVotingFor(ZkappVotingForDiff {
                token,
                public_key: pk,
                voting_for,
                txn_hash,
            }));
        }
    }

    fn expand_actions_diff(
        account_diffs: &mut Vec<AccountDiff>,
        token: TokenAddress,
        pk: PublicKey,
        actions: Vec<ActionState>,
        global_slot: u32,
        txn_hash: TxnHash,
    ) {
        if !actions.is_empty() {
            account_diffs.push(AccountDiff::ZkappActions(ZkappActionsDiff {
                token,
                public_key: pk,
                actions,
                global_slot,
                txn_hash,
            }));
        }
    }

    fn expand_events_diff(
        account_diffs: &mut Vec<AccountDiff>,
        token: TokenAddress,
        pk: PublicKey,
        events: Vec<ZkappEvent>,
        txn_hash: TxnHash,
    ) {
        if !events.is_empty() {
            account_diffs.push(AccountDiff::ZkappEvents(ZkappEventsDiff {
                token,
                public_key: pk,
                events,
                txn_hash,
            }));
        }
    }
}

impl ZkappPaymentDiff {
    pub fn public_key(&self) -> PublicKey {
        match self {
            Self::Payment { payment, .. } => payment.public_key.to_owned(),
            Self::IncrementNonce(diff) => diff.public_key.to_owned(),
        }
    }

    pub fn token(&self) -> TokenAddress {
        match self {
            Self::Payment { payment, .. } => payment
                .token
                .to_owned()
                .unwrap_or_else(TokenAddress::default),
            Self::IncrementNonce(diff) => diff.token.to_owned(),
        }
    }

    pub fn balance_change(&self) -> i64 {
        match self {
            Self::Payment { payment, .. } => payment.balance_change(),
            Self::IncrementNonce(_) => 0.into(),
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

///////////
// debug //
///////////

impl std::fmt::Debug for ZkappFeePayerNonceDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} | {} | {}",
            self.public_key, self.nonce, self.txn_hash
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        base::public_key::PublicKey,
        block::precomputed::{PcbVersion, PrecomputedBlock},
        command::TxnHash,
        ledger::{
            account::{Permission, Permissions},
            diff::{
                account::{
                    zkapp::{
                        ZkappDiff, ZkappEventsDiff, ZkappFeePayerNonceDiff,
                        ZkappIncrementNonceDiff, ZkappPaymentDiff, ZkappPermissionsDiff,
                        ZkappStateDiff, ZkappTokenSymbolDiff, ZkappUriDiff,
                        ZkappVerificationKeyDiff,
                    },
                    AccountDiff, PaymentDiff, UpdateType,
                },
                LedgerDiff,
            },
            token::TokenAddress,
        },
        mina_blocks::v2::zkapp::verification_key::VerificationKey,
    };
    use anyhow::Result;
    use std::path::PathBuf;

    #[test]
    #[allow(clippy::too_many_lines)]
    fn zkapp_account_diff() -> Result<()> {
        let path = PathBuf::from("./tests/data/misc_blocks/mainnet-359630-3NLjRmTyUzeA7meRAT3Yjqxzfe95GKBgkLPD2iLeVE5RMCFcw8eL.json");
        let pcb = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;

        use AccountDiff::*;
        let global_slot = pcb.global_slot_since_genesis();

        // zkapp ledger diffs
        let diffs = LedgerDiff::from_precomputed_unexpanded(&pcb);
        assert_eq!(
            diffs.state_hash.0,
            "3NLjRmTyUzeA7meRAT3Yjqxzfe95GKBgkLPD2iLeVE5RMCFcw8eL"
        );

        let zkapp_diffs = diffs.filter_zkapp();

        // expected unexpanded zkapp account diffs
        let expect = vec![
            vec![
                ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    nonce: 185.into(),
                    txn_hash: TxnHash::new("5JupzBGPDgf3bmLT34EF1uWDDUQNcsgEx8jxNFLRC64da1WkeFaP")?,
                }),
                Zkapp(Box::new(ZkappDiff {
                    nonce: None,
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    payment_diffs: vec![ZkappPaymentDiff::Payment {
                        creation_fee_paid: false,
                        payment: PaymentDiff {
                            public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5"
                                .into(),
                            update_type: UpdateType::Debit(None),
                            amount: 2000000000.into(),
                            txn_hash: TxnHash::new("5JupzBGPDgf3bmLT34EF1uWDDUQNcsgEx8jxNFLRC64da1WkeFaP").ok(),
                            token: Some(TokenAddress::default()),
                        }}
                    ],
                    txn_hash: TxnHash::new("5JupzBGPDgf3bmLT34EF1uWDDUQNcsgEx8jxNFLRC64da1WkeFaP")?,
                    global_slot,
                    ..Default::default()
                })),
                Zkapp(Box::new(ZkappDiff {
                    nonce: None,
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    payment_diffs: vec![ZkappPaymentDiff::Payment {
                        creation_fee_paid: false,
                        payment: PaymentDiff {
                            public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5"
                                .into(),
                            update_type: UpdateType::Credit,
                            amount: 2000000000.into(),
                            txn_hash: TxnHash::new("5JupzBGPDgf3bmLT34EF1uWDDUQNcsgEx8jxNFLRC64da1WkeFaP").ok(),
                            token: Some(TokenAddress::default()),
                        }}
                    ],
                    txn_hash: TxnHash::new("5JupzBGPDgf3bmLT34EF1uWDDUQNcsgEx8jxNFLRC64da1WkeFaP")?,
                    global_slot,
                    ..Default::default()
                })),
            ],
            vec![
                ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    nonce: 186.into(),
                    txn_hash: TxnHash::new("5JtwZTciHh2vrkNUxAt9UexutRURsXVs4N6ifXQrUq3psVcb1WAR")?,
                }),
                Zkapp(Box::new(ZkappDiff {
                    nonce: None,
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    payment_diffs: vec![ZkappPaymentDiff::Payment {
                        creation_fee_paid: false,
                        payment: PaymentDiff {
                            public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5"
                                .into(),
                            update_type: UpdateType::Debit(None),
                            amount: 2000000000.into(),
                            txn_hash: TxnHash::new("5JtwZTciHh2vrkNUxAt9UexutRURsXVs4N6ifXQrUq3psVcb1WAR").ok(),
                            token: Some(TokenAddress::default()),
                        }}
                    ],
                    global_slot,
                    txn_hash: TxnHash::new("5JtwZTciHh2vrkNUxAt9UexutRURsXVs4N6ifXQrUq3psVcb1WAR")?,
                    ..Default::default()
                })),
                Zkapp(Box::new(ZkappDiff {
                    nonce: None,
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    payment_diffs: vec![ZkappPaymentDiff::Payment {
                        creation_fee_paid: false,
                        payment: PaymentDiff {
                            public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5"
                                .into(),
                            update_type: UpdateType::Credit,
                            amount: 2000000000.into(),
                            txn_hash: TxnHash::new("5JtwZTciHh2vrkNUxAt9UexutRURsXVs4N6ifXQrUq3psVcb1WAR").ok(),
                            token: Some(TokenAddress::default()),
                        }}
                    ],
                    global_slot,
                    txn_hash: TxnHash::new("5JtwZTciHh2vrkNUxAt9UexutRURsXVs4N6ifXQrUq3psVcb1WAR")?,
                    ..Default::default()
                })),
            ],
            vec![
                ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    nonce: 187.into(),
                    txn_hash: TxnHash::new("5JtW512WGcQTUMn9gUX2Nq7WL2DjbeQswWt6EY6SkqeiGacbawQG")?,
                }),
                Zkapp(Box::new(ZkappDiff {
                    nonce: None,
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    payment_diffs: vec![ZkappPaymentDiff::Payment {
                        creation_fee_paid: false,
                        payment: PaymentDiff {
                            public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5"
                                .into(),
                            update_type: UpdateType::Debit(None),
                            amount: 2000000000.into(),
                            txn_hash: TxnHash::new("5JtW512WGcQTUMn9gUX2Nq7WL2DjbeQswWt6EY6SkqeiGacbawQG").ok(),
                            token: Some(TokenAddress::default()),
                        }}
                    ],
                    txn_hash: TxnHash::new("5JtW512WGcQTUMn9gUX2Nq7WL2DjbeQswWt6EY6SkqeiGacbawQG")?,
                    global_slot,
                    ..Default::default()
                })),
                Zkapp(Box::new(ZkappDiff {
                    nonce: None,
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    payment_diffs: vec![ZkappPaymentDiff::Payment {
                        creation_fee_paid: false,
                        payment: PaymentDiff {
                            public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5"
                                .into(),
                            update_type: UpdateType::Credit,
                            amount: 2000000000.into(),
                            txn_hash: TxnHash::new("5JtW512WGcQTUMn9gUX2Nq7WL2DjbeQswWt6EY6SkqeiGacbawQG").ok(),
                            token: Some(TokenAddress::default()),
                        }}
                    ],
                    txn_hash: TxnHash::new("5JtW512WGcQTUMn9gUX2Nq7WL2DjbeQswWt6EY6SkqeiGacbawQG")?,
                    global_slot,
                    ..Default::default()
                })),
            ],
            vec![
                ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                    public_key: "B62qoxZPhqRsKromMF72kjZr6LQnufZ8T2iZuDzCmtuDnnddCRF7fpp".into(),
                    nonce: 5.into(),
                    txn_hash: TxnHash::new("5JuTH1NmvdZCG5Ko5rT6PC8zLsJVo6rXjEvNU5whwEgbBuoK6Ltm")?,
                }),
                Zkapp(Box::new(ZkappDiff {
                    nonce: None,
                    public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".into(),
                    payment_diffs: vec![
                        ZkappPaymentDiff::IncrementNonce(ZkappIncrementNonceDiff {
                            public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
                                .into(),
                            token: TokenAddress::default(),
                            creation_fee_paid: false,
                            txn_hash: TxnHash::new("5JuTH1NmvdZCG5Ko5rT6PC8zLsJVo6rXjEvNU5whwEgbBuoK6Ltm")?,
                        }),
                        ZkappPaymentDiff::Payment {
                            creation_fee_paid: false,
                            payment: PaymentDiff {
                                amount: 0.into(),
                                update_type: UpdateType::Credit,
                                public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
                                    .into(),
                                txn_hash: TxnHash::new("5JuTH1NmvdZCG5Ko5rT6PC8zLsJVo6rXjEvNU5whwEgbBuoK6Ltm").ok(),
                                token: Some(TokenAddress::default()),
                            }
                        },
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
                    txn_hash: TxnHash::new("5JuTH1NmvdZCG5Ko5rT6PC8zLsJVo6rXjEvNU5whwEgbBuoK6Ltm")?,
                    zkapp_uri: Some("https://minainu.com".into()),
                    token_symbol: Some("MINU".into()),
                    increment_nonce: true,
                    global_slot,
                    ..Default::default()
                }))
            ],
        ];

        for (n, x) in expect.iter().enumerate() {
            for (m, x) in x.iter().enumerate() {
                assert_eq!(zkapp_diffs[n][m], *x, "n = {n}, m = {m}")
            }
        }
        assert_eq!(zkapp_diffs, expect);

        // expected expanded zkapp diffs
        let expect = vec![
            vec![
                ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    nonce: 185.into(),
                    txn_hash: TxnHash::new("5JupzBGPDgf3bmLT34EF1uWDDUQNcsgEx8jxNFLRC64da1WkeFaP")?,
                }),
                ZkappPayment(ZkappPaymentDiff::Payment {
                    creation_fee_paid: false,
                    payment: PaymentDiff {
                        public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                        update_type: UpdateType::Debit(None),
                        amount: 2000000000.into(),
                        txn_hash: TxnHash::new("5JupzBGPDgf3bmLT34EF1uWDDUQNcsgEx8jxNFLRC64da1WkeFaP").ok(),
                        token: Some(TokenAddress::default()),
                    }}),
                ZkappPayment(ZkappPaymentDiff::Payment {
                    creation_fee_paid: false,
                    payment: PaymentDiff {
                        public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                        update_type: UpdateType::Credit,
                        amount: 2000000000.into(),
                        txn_hash: TxnHash::new("5JupzBGPDgf3bmLT34EF1uWDDUQNcsgEx8jxNFLRC64da1WkeFaP").ok(),
                        token: Some(TokenAddress::default()),
                    }}),
            ],
            vec![
                ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    nonce: 186.into(),
                    txn_hash: TxnHash::new("5JtwZTciHh2vrkNUxAt9UexutRURsXVs4N6ifXQrUq3psVcb1WAR")?,
                }),
                ZkappPayment(ZkappPaymentDiff::Payment {
                    creation_fee_paid: false,
                    payment: PaymentDiff {
                        public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                        update_type: UpdateType::Debit(None),
                        amount: 2000000000.into(),
                        txn_hash: TxnHash::new("5JtwZTciHh2vrkNUxAt9UexutRURsXVs4N6ifXQrUq3psVcb1WAR").ok(),
                        token: Some(TokenAddress::default()),
                    }}),
                ZkappPayment(ZkappPaymentDiff::Payment {
                    creation_fee_paid: false,
                    payment: PaymentDiff {
                        public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                        update_type: UpdateType::Credit,
                        amount: 2000000000.into(),
                        txn_hash: TxnHash::new("5JtwZTciHh2vrkNUxAt9UexutRURsXVs4N6ifXQrUq3psVcb1WAR").ok(),
                        token: Some(TokenAddress::default()),
                    }}),
            ],
            vec![
                ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                    public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                    nonce: 187.into(),
                    txn_hash: TxnHash::new("5JtW512WGcQTUMn9gUX2Nq7WL2DjbeQswWt6EY6SkqeiGacbawQG")?,
                }),
                ZkappPayment(ZkappPaymentDiff::Payment {
                    creation_fee_paid: false,
                    payment: PaymentDiff {
                        public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                        update_type: UpdateType::Debit(None),
                        amount: 2000000000.into(),
                        txn_hash: TxnHash::new("5JtW512WGcQTUMn9gUX2Nq7WL2DjbeQswWt6EY6SkqeiGacbawQG").ok(),
                        token: Some(TokenAddress::default()),
                    }}),
                ZkappPayment(ZkappPaymentDiff::Payment {
                    creation_fee_paid: false,
                    payment: PaymentDiff {
                        public_key: "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5".into(),
                        update_type: UpdateType::Credit,
                        amount: 2000000000.into(),
                        txn_hash: TxnHash::new("5JtW512WGcQTUMn9gUX2Nq7WL2DjbeQswWt6EY6SkqeiGacbawQG").ok(),
                        token: Some(TokenAddress::default()),
                    }}),
            ],
            vec![
                ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                    public_key: "B62qoxZPhqRsKromMF72kjZr6LQnufZ8T2iZuDzCmtuDnnddCRF7fpp".into(),
                    nonce: 5.into(),
                    txn_hash: TxnHash::new("5JuTH1NmvdZCG5Ko5rT6PC8zLsJVo6rXjEvNU5whwEgbBuoK6Ltm")?,
                }),
                ZkappIncrementNonce(ZkappIncrementNonceDiff {
                    public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".into(),
                    token: TokenAddress::default(),
                    creation_fee_paid: false,
                    txn_hash: TxnHash::new("5JuTH1NmvdZCG5Ko5rT6PC8zLsJVo6rXjEvNU5whwEgbBuoK6Ltm")?,
                }),
                ZkappPayment(ZkappPaymentDiff::Payment {
                    creation_fee_paid: false,
                    payment: PaymentDiff {
                        public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".into(),
                        update_type: UpdateType::Credit,
                        amount: 0.into(),
                        txn_hash: TxnHash::new("5JuTH1NmvdZCG5Ko5rT6PC8zLsJVo6rXjEvNU5whwEgbBuoK6Ltm").ok(),
                        token: Some(TokenAddress::default()),
                    }}),
                ZkappVerificationKey(ZkappVerificationKeyDiff {
                    token: TokenAddress::default(),
                    public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".into(),
                    verification_key: VerificationKey {
                        data: "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".into(),
                        hash: "0x1C9320E5FD23AF1F8D8B1145484181C3E6B0F1C8C24FE4BDFFEF4281A61C3EBC".into(),
                    },
                    txn_hash: TxnHash::new("5JuTH1NmvdZCG5Ko5rT6PC8zLsJVo6rXjEvNU5whwEgbBuoK6Ltm")?,
                }),
                ZkappPermissions(ZkappPermissionsDiff {
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
                    },
                    txn_hash: TxnHash::new("5JuTH1NmvdZCG5Ko5rT6PC8zLsJVo6rXjEvNU5whwEgbBuoK6Ltm")?,
                }),
                ZkappUri(ZkappUriDiff {
                    token: TokenAddress::default(),
                    public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".into(),
                    zkapp_uri: "https://minainu.com".into(),
                    txn_hash: TxnHash::new("5JuTH1NmvdZCG5Ko5rT6PC8zLsJVo6rXjEvNU5whwEgbBuoK6Ltm")?,
                }),
                ZkappTokenSymbol(ZkappTokenSymbolDiff {
                    token: TokenAddress::default(),
                    public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".into(),
                    token_symbol: "MINU".into(),
                    txn_hash: TxnHash::new("5JuTH1NmvdZCG5Ko5rT6PC8zLsJVo6rXjEvNU5whwEgbBuoK6Ltm")?,
                }),
            ],
        ];

        let expanded = AccountDiff::expand(zkapp_diffs);
        for (n, x) in expect.iter().enumerate() {
            for (m, x) in x.iter().enumerate() {
                assert_eq!(expanded[n][m], *x, "n = {n}, m = {m}")
            }
        }
        assert_eq!(expanded, expect);

        Ok(())
    }

    #[test]
    fn zkapp_account_diff_new_token() -> Result<()> {
        let path = PathBuf::from("./tests/data/misc_blocks/mainnet-360930-3NL3mVAEwJuBS8F3fMWBZZRjQC4JBzdGTD7vN5SqizudnkPKsRyi.json");
        let pcb = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;
        let ledger_diff = LedgerDiff::from_precomputed(&pcb);

        use AccountDiff::*;

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
        let fee_payer: PublicKey = "B62qo69VLUPMXEC6AFWRgjdTEGsA3xKvqeU5CgYm3jAbBJL7dTvaQkv".into();
        let token =
            TokenAddress::new("xosVXFFDvDiKvHSDAaHvrTSRtoa5Graf2J7LM5Smb4GNTrT2Hn").unwrap();

        let expect = vec![vec![
            ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                public_key: fee_payer.to_owned(),
                nonce: 1.into(),
                txn_hash: TxnHash::new("5Juph2RjJTR4rF2u9mKzWTS9wze7romYLaH1RqRLKN7eLbecXZX1")?,
            }),
            ZkappPayment(ZkappPaymentDiff::Payment {
                creation_fee_paid: true,
                payment: PaymentDiff {
                    public_key: fee_payer.clone(),
                    update_type: UpdateType::Debit(None),
                    amount: 1000000000.into(),
                    txn_hash: TxnHash::new("5Juph2RjJTR4rF2u9mKzWTS9wze7romYLaH1RqRLKN7eLbecXZX1").ok(),
                    token: Some(TokenAddress::default()),
                }}),
            ZkappPayment(ZkappPaymentDiff::Payment {
                creation_fee_paid: true,
                payment: PaymentDiff {
                    public_key: "B62qnzkHunByjReoEwMKCJ9HQxZP2MSYcUe8Lfesy4SpufxWp3viNFT".into(),
                    update_type: UpdateType::Credit,
                    amount: 0.into(),
                    txn_hash: TxnHash::new("5Juph2RjJTR4rF2u9mKzWTS9wze7romYLaH1RqRLKN7eLbecXZX1").ok(),
                    token: Some(TokenAddress::default()),
                }}),
            ZkappEvents(ZkappEventsDiff {
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
                ],
                txn_hash: TxnHash::new("5Juph2RjJTR4rF2u9mKzWTS9wze7romYLaH1RqRLKN7eLbecXZX1")?,
            }),
            ZkappPayment(ZkappPaymentDiff::Payment {
                creation_fee_paid: true,
                payment: PaymentDiff {
                    public_key: fee_payer.clone(),
                    update_type: UpdateType::Credit,
                    amount: 0.into(),
                    txn_hash: TxnHash::new("5Juph2RjJTR4rF2u9mKzWTS9wze7romYLaH1RqRLKN7eLbecXZX1").ok(),
                    token: Some(TokenAddress::default()),
                }}),
            ZkappPayment(ZkappPaymentDiff::Payment {
                creation_fee_paid: true,
                payment: PaymentDiff {
                    public_key: fee_payer.clone(),
                    update_type: UpdateType::Debit(None),
                    amount: 19000000000.into(),
                    txn_hash: TxnHash::new("5Juph2RjJTR4rF2u9mKzWTS9wze7romYLaH1RqRLKN7eLbecXZX1").ok(),
                    token: Some(TokenAddress::default()),
                }}),
            ZkappPayment(ZkappPaymentDiff::Payment {
                creation_fee_paid: true,
                payment: PaymentDiff {
                    public_key: "B62qq7ecvBQZQK68dwstL27888NEKZJwNXNFjTyu3xpQcfX5UBivCU6".into(),
                    update_type: UpdateType::Credit,
                    amount: 19000000000.into(),
                    txn_hash: TxnHash::new("5Juph2RjJTR4rF2u9mKzWTS9wze7romYLaH1RqRLKN7eLbecXZX1").ok(),
                    token: Some(TokenAddress::default()),
                }}),
            ZkappIncrementNonce(ZkappIncrementNonceDiff {
                public_key: "B62qnVLedrzTUMZME91WKbNw3qJ3hw7cc5PeK6RR3vH7RTFTsVbiBj4".into(),
                token: token.clone(),
                creation_fee_paid: true,
                txn_hash: TxnHash::new("5Juph2RjJTR4rF2u9mKzWTS9wze7romYLaH1RqRLKN7eLbecXZX1")?,
            }),
            ZkappPayment(ZkappPaymentDiff::Payment {
                creation_fee_paid: true,
                payment: PaymentDiff {
                    public_key: "B62qnVLedrzTUMZME91WKbNw3qJ3hw7cc5PeK6RR3vH7RTFTsVbiBj4".into(),
                    update_type: UpdateType::Credit,
                    amount: 0.into(),
                    token: TokenAddress::new("xosVXFFDvDiKvHSDAaHvrTSRtoa5Graf2J7LM5Smb4GNTrT2Hn"),
                    txn_hash: TxnHash::new("5Juph2RjJTR4rF2u9mKzWTS9wze7romYLaH1RqRLKN7eLbecXZX1").ok(),
            }}),
            ZkappState(ZkappStateDiff {
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
                txn_hash: TxnHash::new("5Juph2RjJTR4rF2u9mKzWTS9wze7romYLaH1RqRLKN7eLbecXZX1")?,
            }),
            ZkappVerificationKey(ZkappVerificationKeyDiff {
                token: token.clone(),
                public_key: "B62qnVLedrzTUMZME91WKbNw3qJ3hw7cc5PeK6RR3vH7RTFTsVbiBj4".into(),
                verification_key: VerificationKey {
                    data: "zBpGMTFexQVgZ8eDUUxauEXpFtAerH9Vq9y7enWrcqJEhmPfMFLcKiu16J3nMcTBaKtAZZEDDcUKLFNCdyhzBsTx2S3zKpjAzM3w5cEVJLUbZsT4UnAZpeiXKpM6qbGbUY6LuWgz6Yn2r5TQA82ojYjFMQHqEJz7koH6iQdbiv1H9TX3ztCR4QVW9vfmF6GpbFNiab14xyDD5t8Wb5ym4od2CwQe6t5ctDSi7h5BgL5xe1Qihk1jXgjYHowErnS9JcU2jsXyNeDrdc2osVuqCL2bhYiuYCzGXed6qqkqqyBckKTYo8UMeoCvyGMxtR5L1kuKAPskEZFE27eCoAQzrKAyqH4tEmMJSJMWZTrdFF6rfvQP3X2JVfk6c3VLXDtQiqJ9YH2Vv6NZ3tCi6evL36CTV3jVyY4qnK2YqLVWcqbUXN2LRVhrpmz8c8VqrKfVvrG1oSGBSPNhnTQeALQVErUZC1xi58TbKfk8BG4vySsPnQLXibXGhYWGRgtzU5Tbg111CBwG3dkzs81HJ5uLwV8A35PtA3kCYxo2eUcvRR5p7QbL5d6RMHLeeKGo5gjyK78oMYmDYgpK3sd5QUw7NbFMzmWTMBhxAhy3NobuhK1SHPWqohozksCod2F52tyjaVhT4qoyjvUDLxd5n5nyG2Xe9n1KcynbZ9B2xK22U7fMRBWJvoQv957K3XrGRFLp8qy6Phgmyo8crWXTrn4veRoWdducAEb39rsoYyxawXQBPsy4WCZLhJ34TMF6KV2EmotigTiW7Vz92W1THgb9qW6rG6Zeb7hWnxBcsjZ3pitcrjZ9Bjyc7gFGfkiTwgm45k9M9aNtg9EuYYZjb2nNy9LBMTCHWWhmFWeNLSRUGYPv5zCyyEw5P8gJsFdPqbcDUPJqq4F9qCTcobTfBt4a9HHc8AExhWzvJuV2AQyvM2KLiPCX93AzvAWehZV2K5qngDtJfFwAFV6PLVBnXPe7jCSBihFy2oDrfkuuAVSgg9DM6rkQAindetWAXTWNRbATv6T7TH4QxAwwsB3Vcw1Dq6EFUhaBdKG3xgv1fSuiNeJMEZCrAVSwR3fKSLuhewiBabEx9pJS7A9K1GVTm8qTrDuVcxrovEiGmLJny6A8Q1G6CyJksbt7LRspBFSq8s3x229AJgJ3XsLn7RW4kUBEWYjEbUH61vQcfcbgCrZsGJtSB8jKgoBR7JfhJe1t3R37wK11weDrZawWcC4zhEgZATr321LFY6gDsaDNruxaMrmUDX1EyP1TtpZgnd4qrSni9cpvceJZxaYkE9wC8uggVRSXc8NgHh2o9ECA6aZHTaGr85uNYis7bLhg7ss7PWzuHsuus1JtXasWMhZa52awY8YXpmuLs1zczpBTA1ZkBh9H3jDkN7eNnpj4gdAmw3rZf3hcQ94p8mvHKNLjJnjZSzy6ovFsVrRc9iyVsfZmwAkhVe9PJVLy9PaRNPn1x3YMLGYvCkw58kxwiZEd549ussEcZsBhpy2RE51jeej5MvT8ruECsqxXVQGeRvaSLWgSgPFwcpW7SMmUTLB9xVb9AMcmPGiADv5UMG7Gw48KxdqdaRUZdaWaFSjUBApRTH7XXa12Jng9UVbgfkRYLYZJfCyZdxBE2uhEMkZh2G81GBLb3N7tnWdf77b6ewef6bEh6mMcJe2bLVuCoNtUbcQsG3CWvPbsEfc47bM42B7Xg3Nc3LmjBHVexLWmch95JjGzYNxUz5t4Nd5oPFSesBwpk6qmLtjkSUR65DzmhbcEN5M7rcDbJXiuWKDwaU5zre2ZfquZQqjzjG3iiaAaaQcUQPPKfjZCetGqXryiJ3i48LgEHvMXqmRxbjtFSHVrgTF4H65qmgDyxHk2QtexLi88X4BeSQP8LeBKkpAKs7e2E4HvsoUACPU1xR2DZxcSJeWnExWZRPEwjXGmp8o1gN6Cjh85xtC1y4ZdwT3ThsG17qoynYsGSRb1MRjBUrs3jWTGjJZoM8Gpm4NAZyBYnGywJRtPJiHHhkx1Adt72bfPRt9kkLsWbniQSL8hJox1z7GT4cVXdxzTn2DTQ5WtkmeNSMFZ1LdJvmGHLekh6sGiifmqQaArG2ZgSPiP6NbtLdkyCQmaJCysv6R3C5ZjTkDx8RSsroBKc2c9RErdjAhSXN7tQrYtgWQGQu1pwT6GK6C492azsuuuNVm2puufuswhXWhLLTR7HmcLiEd3P1DrQvmVcn2KjMmgJVN1EsxVwKtnGTk3kwZYtCLFG5ABddWUpz1o9TdYJGuA2DuoLCM3w9TMwUVGpyvbTNVHDUJjX8MaW7bEzPRRJdMwoEnU2P7mreVa9P4daBEXnYyM1owryckPwD6H2NvkPLFQZo59Dkqdk3iSrp99dxtdLfgLHRdiru8G8sGFP1pGGoZEDhiDBFcenVZYYw1TLwP18pqaEtjgqgTBd13gbM9oVgkasPdq94VGvgp".into(),
                    hash: "0x3ECC0FC66665B96DA4ED0CC9EF3926C359B0EA44D81E537D02051DAD97F49BED".into()
                },
                txn_hash: TxnHash::new("5Juph2RjJTR4rF2u9mKzWTS9wze7romYLaH1RqRLKN7eLbecXZX1")?,
            }),
            ZkappPermissions(ZkappPermissionsDiff {
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
                },
                txn_hash: TxnHash::new("5Juph2RjJTR4rF2u9mKzWTS9wze7romYLaH1RqRLKN7eLbecXZX1")?,
            }),
            ZkappPayment(ZkappPaymentDiff::Payment {
                creation_fee_paid: true,
                payment: PaymentDiff {
                    public_key: "B62qnVLedrzTUMZME91WKbNw3qJ3hw7cc5PeK6RR3vH7RTFTsVbiBj4".into(),
                    update_type: UpdateType::Credit,
                    amount: 1000000000.into(),
                    token: Some(token.clone()),
                    txn_hash: TxnHash::new("5Juph2RjJTR4rF2u9mKzWTS9wze7romYLaH1RqRLKN7eLbecXZX1").ok(),
                }}),
        ]];

        for (n, x) in expect[0].iter().enumerate() {
            assert_eq!(zkapp_diffs[0][n], *x, "n = {n}")
        }
        assert_eq!(zkapp_diffs, expect);

        Ok(())
    }

    #[test]
    fn zkapp_account_diff_359611() -> Result<()> {
        let path = PathBuf::from("./tests/data/hardfork/mainnet-359611-3NKybkb8C3R5PjwkxNUVCL6tb5qVf5i4jPWkDCcyJbka9Qgvr8CG.json");
        let pcb = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;
        let ledger_diff = LedgerDiff::from_precomputed(&pcb);

        use AccountDiff::*;

        let account_diffs = ledger_diff.account_diffs[10][..4].to_vec();
        let expect = vec![
            ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                public_key: "B62qnYLvgjkDpAYc9eaPvDbMmJWKJg8pB5C1Gmte6eJZP2miPs7uiiY".into(),
                nonce: 1.into(),
                txn_hash: TxnHash::new("5JuaVdVXB46z1Njjz4B6eZZ8EpoDGrfsUzoPdB1gNWHTeHnnSVQd")?,
            }),
            ZkappPayment(ZkappPaymentDiff::Payment {
                creation_fee_paid: true,
                payment: PaymentDiff {
                    amount: 1000000000.into(),
                    update_type: UpdateType::Debit(None),
                    public_key: "B62qnYLvgjkDpAYc9eaPvDbMmJWKJg8pB5C1Gmte6eJZP2miPs7uiiY".into(),
                    txn_hash: TxnHash::new("5JuaVdVXB46z1Njjz4B6eZZ8EpoDGrfsUzoPdB1gNWHTeHnnSVQd")
                        .ok(),
                    token: Some(TokenAddress::default()),
                },
            }),
            ZkappIncrementNonce(ZkappIncrementNonceDiff {
                public_key: "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P".into(),
                token: TokenAddress::default(),
                creation_fee_paid: true,
                txn_hash: TxnHash::new("5JuaVdVXB46z1Njjz4B6eZZ8EpoDGrfsUzoPdB1gNWHTeHnnSVQd")?,
            }),
            ZkappPayment(ZkappPaymentDiff::Payment {
                creation_fee_paid: true,
                payment: PaymentDiff {
                    amount: 0.into(),
                    update_type: UpdateType::Credit,
                    public_key: "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P".into(),
                    txn_hash: TxnHash::new("5JuaVdVXB46z1Njjz4B6eZZ8EpoDGrfsUzoPdB1gNWHTeHnnSVQd")
                        .ok(),
                    token: Some(TokenAddress::default()),
                },
            }),
        ];

        for (n, x) in expect.iter().enumerate() {
            assert_eq!(
                account_diffs[n], *x,
                "n = {}\nGOT: {:#?}\nEXPECT: {:#?}",
                n, account_diffs[n], x
            );
        }

        assert_eq!(account_diffs, expect);
        Ok(())
    }

    #[test]
    fn zkapp_account_diff_359617() -> Result<()> {
        let path = PathBuf::from("./tests/data/hardfork/mainnet-359617-3NKZ5poCAjtGqg9hHvAVZ7QwriqJsL8mpQsSHFGzqW6ddEEjYfvW.json");
        let pcb = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;
        let ledger_diff = LedgerDiff::from_precomputed(&pcb);

        use AccountDiff::*;

        let account_diffs = ledger_diff.account_diffs[7][..4].to_vec();
        let expect = vec![
            ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                public_key: "B62qoxZPhqRsKromMF72kjZr6LQnufZ8T2iZuDzCmtuDnnddCRF7fpp".into(),
                nonce: 4.into(),
                txn_hash: TxnHash::new("5Jui9AUvGSsqYhMsXMLgV28UkPiXcNiuGZXLMTWqrstGAkocAoAi")?,
            }),
            ZkappPayment(ZkappPaymentDiff::Payment {
                creation_fee_paid: true,
                payment: PaymentDiff {
                    amount: 2000000000.into(),
                    update_type: UpdateType::Debit(None),
                    public_key: "B62qoxZPhqRsKromMF72kjZr6LQnufZ8T2iZuDzCmtuDnnddCRF7fpp".into(),
                    txn_hash: TxnHash::new("5Jui9AUvGSsqYhMsXMLgV28UkPiXcNiuGZXLMTWqrstGAkocAoAi")
                        .ok(),
                    token: Some(TokenAddress::default()),
                },
            }),
            ZkappIncrementNonce(ZkappIncrementNonceDiff {
                creation_fee_paid: true,
                public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".into(),
                txn_hash: TxnHash::new("5Jui9AUvGSsqYhMsXMLgV28UkPiXcNiuGZXLMTWqrstGAkocAoAi")?,
                token: TokenAddress::default(),
            }),
            ZkappPayment(ZkappPaymentDiff::Payment {
                creation_fee_paid: true,
                payment: PaymentDiff {
                    amount: 0.into(),
                    update_type: UpdateType::Credit,
                    public_key: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF".into(),
                    txn_hash: TxnHash::new("5Jui9AUvGSsqYhMsXMLgV28UkPiXcNiuGZXLMTWqrstGAkocAoAi")
                        .ok(),
                    token: Some(TokenAddress::default()),
                },
            }),
        ];

        for (n, x) in expect.iter().enumerate() {
            assert_eq!(
                account_diffs[n], *x,
                "n = {}\nGOT: {:#?}\nEXPECT: {:#?}",
                n, account_diffs[n], x
            );
        }

        assert_eq!(account_diffs, expect);
        Ok(())
    }

    #[test]
    fn zkapp_account_diff_368442() -> Result<()> {
        let path = PathBuf::from("./tests/data/misc_blocks/mainnet-368442-3NLTFUdvKixsbCqEbjWKskrjWuaSQpwTjoGNXWzK7eaUn4oHscbu.json");
        let pcb = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;
        let ledger_diff = LedgerDiff::from_precomputed(&pcb);

        use AccountDiff::*;

        let account_diffs = ledger_diff.account_diffs[23].clone();
        let expect = vec![
            ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                public_key: "B62qkikiZXisUGspBunSnKQn5FRaUPkLUBbxkBY64Xn6AnaSwgKab5h".into(),
                nonce: 59.into(),
                txn_hash: TxnHash::new("5Jtd7WfXXmhBeDWV3JsW4zRjXVBd2AALm65CPpp9k9JuQ1mhUHE9")?,
            }),
            ZkappPayment(ZkappPaymentDiff::Payment {
                creation_fee_paid: true,
                payment: PaymentDiff {
                    amount: 1000000000.into(),
                    update_type: UpdateType::Debit(None),
                    public_key: "B62qkikiZXisUGspBunSnKQn5FRaUPkLUBbxkBY64Xn6AnaSwgKab5h".into(),
                    txn_hash: TxnHash::new("5Jtd7WfXXmhBeDWV3JsW4zRjXVBd2AALm65CPpp9k9JuQ1mhUHE9")
                        .ok(),
                    token: Some(TokenAddress::default()),
                },
            }),
            ZkappPayment(ZkappPaymentDiff::Payment {
                creation_fee_paid: true,
                payment: PaymentDiff {
                    amount: 0.into(),
                    update_type: UpdateType::Credit,
                    public_key: "B62qjwDWxjf4LtJ4YWJQDdTNPqZ69ZyeCzbpAFKN7EoZzYig5ZRz8JE".into(),
                    txn_hash: TxnHash::new("5Jtd7WfXXmhBeDWV3JsW4zRjXVBd2AALm65CPpp9k9JuQ1mhUHE9")
                        .ok(),
                    token: Some(TokenAddress::default()),
                },
            }),
            ZkappEvents(ZkappEventsDiff {
                token: TokenAddress::default(),
                public_key: "B62qjwDWxjf4LtJ4YWJQDdTNPqZ69ZyeCzbpAFKN7EoZzYig5ZRz8JE".into(),
                events: vec![
                    "0x0000000000000000000000000000000000000000000000000000000000000002".into(),
                    "0x01B2700CB8B5AB3EA1E6901ED662EDBD45F1FADEDFDA70A406F2E36F7A902F2C".into(),
                    "0x0000000000000000000000000000000000000000000000000000000000000001".into(),
                    "0x3D76374FA52F749B664DB992AF45C57F92535C1CDAED68867781673A7E278F78".into(),
                    "0x0000000000000000000000000000000000000000000000000000000000000001".into(),
                    "0x00000000000000000000000000000000000000000000000000000002540BE400".into(),
                ],
                txn_hash: TxnHash::new("5Jtd7WfXXmhBeDWV3JsW4zRjXVBd2AALm65CPpp9k9JuQ1mhUHE9")?,
            }),
            ZkappPayment(ZkappPaymentDiff::Payment {
                creation_fee_paid: true,
                payment: PaymentDiff {
                    amount: 10000000000.into(),
                    update_type: UpdateType::Debit(None),
                    public_key: "B62qjwDWxjf4LtJ4YWJQDdTNPqZ69ZyeCzbpAFKN7EoZzYig5ZRz8JE".into(),
                    token: TokenAddress::new("xBxjFpJkbWpbGua7Lf36S1NLhffFoEChyP3pz6SYKnx7dFCTwg"),
                    txn_hash: TxnHash::new("5Jtd7WfXXmhBeDWV3JsW4zRjXVBd2AALm65CPpp9k9JuQ1mhUHE9")
                        .ok(),
                },
            }),
            ZkappPayment(ZkappPaymentDiff::Payment {
                creation_fee_paid: true,
                payment: PaymentDiff {
                    amount: 10000000000.into(),
                    update_type: UpdateType::Credit,
                    public_key: "B62qnVgC5sXACSeAAYV7wjeLYFeC3XZ1PA2MBsuSUUsqiK96jfN9sba".into(),
                    token: TokenAddress::new("xBxjFpJkbWpbGua7Lf36S1NLhffFoEChyP3pz6SYKnx7dFCTwg"),
                    txn_hash: TxnHash::new("5Jtd7WfXXmhBeDWV3JsW4zRjXVBd2AALm65CPpp9k9JuQ1mhUHE9")
                        .ok(),
                },
            }),
        ];

        for (n, x) in expect.iter().enumerate() {
            assert_eq!(
                account_diffs[n], *x,
                "n = {}\nGOT: {:#?}\nEXPECT: {:#?}",
                n, account_diffs[n], x
            );
        }

        assert_eq!(account_diffs, expect);
        Ok(())
    }

    #[test]
    fn zkapp_account_diff_413047() -> Result<()> {
        let path = PathBuf::from("./tests/data/misc_blocks/mainnet-413047-3NKJ7ts56Hs43ux5uGWi3rsbNvYKsyZbFmFbgRpgUD552M6Dp6xp.json");
        let pcb = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;
        let ledger_diff = LedgerDiff::from_precomputed(&pcb);

        use AccountDiff::*;

        let account_diffs = ledger_diff.account_diffs[24].clone();
        let expect = vec![
            ZkappFeePayerNonce(ZkappFeePayerNonceDiff {
                public_key: "B62qjGsPY47SMkTykivPBAU3riS9gvMMrGr7ve6ynoHJNBzAhQmtoBn".into(),
                nonce: 92.into(),
                txn_hash: TxnHash::new("5JvKj3XZX4SVWj7jBpXPt9FzyiBY4AM24QLUZw8dsqAoGDxkRbxo")?,
            }),
            ZkappPayment(ZkappPaymentDiff::Payment {
                payment: PaymentDiff {
                    amount: 300000000.into(),
                    public_key: "B62qpr8QD2Q9uzJU2pp7XbWW6NB9CQxv4BS6vVXRwwXhcfPwJM7FdCs".into(),
                    update_type: UpdateType::Debit(None),
                    txn_hash: TxnHash::new("5JvKj3XZX4SVWj7jBpXPt9FzyiBY4AM24QLUZw8dsqAoGDxkRbxo")
                        .ok(),
                    token: Some(TokenAddress::default()),
                },
                creation_fee_paid: false,
            }),
            ZkappState(ZkappStateDiff {
                token: TokenAddress::default(),
                public_key: "B62qpr8QD2Q9uzJU2pp7XbWW6NB9CQxv4BS6vVXRwwXhcfPwJM7FdCs".into(),
                diffs: [
                    None,
                    None,
                    None,
                    Some(
                        "0x3FDAEB7767E78BA83FECB4AD8CB34299A4D5C28BAC17D23A9832656BB5D4262C".into(),
                    ),
                    None,
                    Some(
                        "0x00000000000000000000000000000000000000000000000000000000A0EEBB00".into(),
                    ),
                    Some(
                        "0x0000000000000000000000070000000500000003000000010000000100000001".into(),
                    ),
                    Some(
                        "0x00000000000000000000000000000000000000000000000000000000000000B4".into(),
                    ),
                ],
                txn_hash: TxnHash::new("5JvKj3XZX4SVWj7jBpXPt9FzyiBY4AM24QLUZw8dsqAoGDxkRbxo")?,
            }),
            ZkappEvents(ZkappEventsDiff {
                token: TokenAddress::default(),
                public_key: "B62qpr8QD2Q9uzJU2pp7XbWW6NB9CQxv4BS6vVXRwwXhcfPwJM7FdCs".into(),
                events: vec![
                    "0x0000000000000000000000000000000000000000000000000000000000000003".into(),
                    "0x0000000000000000000000070000000500000003000000010000000100000001".into(),
                    "0x00000000000000000000000000000000000000000000000000000000000000B4".into(),
                    "0x00000000000000000000000000000000000000000000000000000000A0EEBB00".into(),
                ],
                txn_hash: TxnHash::new("5JvKj3XZX4SVWj7jBpXPt9FzyiBY4AM24QLUZw8dsqAoGDxkRbxo")?,
            }),
            ZkappPayment(ZkappPaymentDiff::Payment {
                payment: PaymentDiff {
                    amount: 0.into(),
                    public_key: "B62qrcJEepeTxGXiBcPwGc2hfN358Zh4rVF2w6Uv37j4tK3jTSajDRX".into(),
                    update_type: UpdateType::Credit,
                    txn_hash: TxnHash::new("5JvKj3XZX4SVWj7jBpXPt9FzyiBY4AM24QLUZw8dsqAoGDxkRbxo")
                        .ok(),
                    token: Some(TokenAddress::default()),
                },
                creation_fee_paid: false,
            }),
            ZkappPayment(ZkappPaymentDiff::Payment {
                payment: PaymentDiff {
                    amount: 300000000.into(),
                    public_key: "B62qm9d3Ff7DQMpc59wNv9d6R9mSqRKbtHsPs53ZBGr27Y7Cj1poEmc".into(),
                    update_type: UpdateType::Credit,
                    txn_hash: TxnHash::new("5JvKj3XZX4SVWj7jBpXPt9FzyiBY4AM24QLUZw8dsqAoGDxkRbxo")
                        .ok(),
                    token: Some(TokenAddress::default()),
                },
                creation_fee_paid: false,
            }),
        ];

        for (n, x) in expect.iter().enumerate() {
            assert_eq!(
                account_diffs[n], *x,
                "n = {}\nGOT: {:#?}\nEXPECT: {:#?}",
                n, account_diffs[n], x
            );
        }

        assert_eq!(account_diffs, expect);
        Ok(())
    }
}
