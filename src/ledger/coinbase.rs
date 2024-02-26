use crate::{
    block::precomputed::PrecomputedBlock,
    ledger::{diff::account::AccountDiff, PublicKey},
    protocol::serialization_types::staged_ledger_diff::{CoinBase, CoinBaseFeeTransfer},
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Coinbase {
    pub kind: CoinbaseKind,
    pub receiver: PublicKey,
    supercharge: bool,
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

impl From<CoinBaseFeeTransfer> for CoinbaseFeeTransfer {
    fn from(value: CoinBaseFeeTransfer) -> Self {
        Self {
            receiver_pk: value.receiver_pk.into(),
            fee: value.fee.inner().inner(),
        }
    }
}

impl CoinbaseKind {
    pub fn from_precomputed_block(precomputed_block: &PrecomputedBlock) -> Self {
        match precomputed_block.staged_ledger_pre_diff().coinbase.inner() {
            CoinBase::Zero => Self::Zero,
            CoinBase::One(x) => Self::One(x.map(|cb| {
                let CoinBaseFeeTransfer { receiver_pk, fee } = cb.inner().inner();
                CoinbaseFeeTransfer {
                    receiver_pk: receiver_pk.into(),
                    fee: fee.inner().inner(),
                }
            })),
            CoinBase::Two(x, y) => Self::Two(
                x.map(|c| c.inner().inner().into()),
                y.map(|c| c.inner().inner().into()),
            ),
        }
    }
}

impl Coinbase {
    pub fn from_precomputed(precomputed_block: &PrecomputedBlock) -> Self {
        let consensus_state = precomputed_block.consensus_state();
        let receiver: PublicKey = consensus_state.coinbase_receiver.into();
        let supercharge = consensus_state.supercharge_coinbase;
        let kind = CoinbaseKind::from_precomputed_block(precomputed_block);

        Self {
            kind,
            receiver,
            supercharge,
        }
    }

    pub fn is_coinbase_applied(&self) -> bool {
        !matches!(self.kind, CoinbaseKind::Zero)
    }

    // only apply if "coinbase" =/= [ "Zero" ]
    pub fn as_account_diff(self) -> Option<AccountDiff> {
        if self.is_coinbase_applied() {
            return Some(AccountDiff::from_coinbase(self.receiver, self.supercharge));
        }
        None
    }
}
