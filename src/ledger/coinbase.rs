use crate::{
    block::precomputed::PrecomputedBlock,
    ledger::{
        diff::account::{AccountDiff, PaymentDiff, UpdateType},
        PublicKey,
    },
    protocol::serialization_types::staged_ledger_diff,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Coinbase {
    pub kind: CoinbaseKind,
    pub receiver: PublicKey,
    pub supercharge: bool,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CoinbaseKind {
    Zero,
    One(Option<CoinbaseFeeTransfer>),
    Two(Option<CoinbaseFeeTransfer>, Option<CoinbaseFeeTransfer>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CoinbaseFeeTransfer {
    pub receiver_pk: PublicKey,
    pub fee: u64,
}

impl From<staged_ledger_diff::CoinBaseFeeTransfer> for CoinbaseFeeTransfer {
    fn from(value: staged_ledger_diff::CoinBaseFeeTransfer) -> Self {
        Self {
            receiver_pk: PublicKey::from(value.receiver_pk),
            fee: value.fee.inner().inner(),
        }
    }
}

impl CoinbaseKind {
    pub fn from_precomputed(precomputed_block: &PrecomputedBlock) -> Vec<Self> {
        let mut res = vec![];
        let pre_diff_coinbase = match precomputed_block.staged_ledger_pre_diff().coinbase.inner() {
            staged_ledger_diff::CoinBase::Zero => Self::Zero,
            staged_ledger_diff::CoinBase::One(x) => Self::One(x.map(|cb| {
                let staged_ledger_diff::CoinBaseFeeTransfer { receiver_pk, fee } =
                    cb.inner().inner();
                CoinbaseFeeTransfer {
                    receiver_pk: PublicKey::from(receiver_pk),
                    fee: fee.inner().inner(),
                }
            })),
            staged_ledger_diff::CoinBase::Two(x, y) => Self::Two(
                x.map(|c| c.inner().inner().into()),
                y.map(|c| c.inner().inner().into()),
            ),
        };
        let post_diff_coinbase = match precomputed_block
            .staged_ledger_post_diff()
            .map(|diff| diff.coinbase.inner())
        {
            None => None,
            Some(staged_ledger_diff::CoinBase::Zero) => Some(Self::Zero),
            Some(staged_ledger_diff::CoinBase::One(x)) => Some(Self::One(x.map(|cb| {
                let staged_ledger_diff::CoinBaseFeeTransfer { receiver_pk, fee } =
                    cb.inner().inner();
                CoinbaseFeeTransfer {
                    receiver_pk: PublicKey::from(receiver_pk),
                    fee: fee.inner().inner(),
                }
            }))),
            Some(staged_ledger_diff::CoinBase::Two(x, y)) => Some(Self::Two(
                x.map(|c| c.inner().inner().into()),
                y.map(|c| c.inner().inner().into()),
            )),
        };

        res.push(pre_diff_coinbase);
        if let Some(post_diff_coinbase) = post_diff_coinbase {
            res.push(post_diff_coinbase);
        }
        res
    }
}

impl Coinbase {
    pub fn from_precomputed(precomputed_block: &PrecomputedBlock) -> Self {
        let consensus_state = precomputed_block.consensus_state();
        let receiver: PublicKey = consensus_state.coinbase_receiver.into();
        let supercharge = consensus_state.supercharge_coinbase;
        let kind = CoinbaseKind::from_precomputed(precomputed_block);
        let kind = if kind.len() < 2 {
            kind[0].clone()
        } else {
            kind[1].clone()
        };

        Self {
            kind,
            receiver,
            supercharge,
        }
    }

    // For fee_transfer_via_coinbase, remove the original fee_trasnfer for SNARK
    // work
    pub fn fee_transfer(&self) -> Vec<PaymentDiff> {
        match &self.kind {
            CoinbaseKind::Zero => vec![],
            CoinbaseKind::One(fee_transfer) => {
                if let Some(fee_transfer) = fee_transfer {
                    vec![
                        PaymentDiff {
                            public_key: self.receiver.clone(),
                            amount: fee_transfer.fee.into(),
                            update_type: UpdateType::Debit(None),
                        },
                        PaymentDiff {
                            public_key: fee_transfer.receiver_pk.clone(),
                            amount: fee_transfer.fee.into(),
                            update_type: UpdateType::Credit,
                        },
                    ]
                } else {
                    vec![]
                }
            }
            CoinbaseKind::Two(fee_transfer0, fee_transfer1) => {
                let mut res = vec![];
                if let Some(t0) = fee_transfer0 {
                    res.append(&mut vec![
                        PaymentDiff {
                            public_key: self.receiver.clone(),
                            amount: t0.fee.into(),
                            update_type: UpdateType::Debit(None),
                        },
                        PaymentDiff {
                            public_key: t0.receiver_pk.clone(),
                            amount: t0.fee.into(),
                            update_type: UpdateType::Credit,
                        },
                    ]);
                }
                if let Some(t1) = fee_transfer1 {
                    res.append(&mut vec![
                        PaymentDiff {
                            public_key: self.receiver.clone(),
                            amount: t1.fee.into(),
                            update_type: UpdateType::Debit(None),
                        },
                        PaymentDiff {
                            public_key: t1.receiver_pk.clone(),
                            amount: t1.fee.into(),
                            update_type: UpdateType::Credit,
                        },
                    ]);
                }
                res
            }
        }
    }

    pub fn is_coinbase_applied(&self) -> bool {
        !matches!(self.kind, CoinbaseKind::Zero)
    }

    pub fn has_fee_transfer(&self) -> bool {
        matches!(
            self.kind,
            CoinbaseKind::One(Some(_))
                | CoinbaseKind::Two(Some(_), _)
                | CoinbaseKind::Two(_, Some(_))
        )
    }

    // only apply if "coinbase" =/= [ "Zero" ]
    pub fn as_account_diff(self) -> Vec<AccountDiff> {
        let mut res = vec![];
        if self.is_coinbase_applied() {
            res.append(&mut AccountDiff::from_coinbase(self));
        }
        res
    }
}
