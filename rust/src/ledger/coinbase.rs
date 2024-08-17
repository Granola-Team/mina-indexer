use crate::{
    block::precomputed::PrecomputedBlock,
    command::internal::InternalCommand,
    constants::*,
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
    pub is_new_account: bool,
    pub receiver_balance: Option<u64>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CoinbaseKind {
    None,
    Coinbase(Option<CoinbaseFeeTransfer>),
    CoinbaseAndFeeTransferViaCoinbase(Option<CoinbaseFeeTransfer>, Option<CoinbaseFeeTransfer>),
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
            staged_ledger_diff::CoinBase::None => Self::None,
            staged_ledger_diff::CoinBase::Coinbase(x) => Self::Coinbase(x.map(|cb| {
                let staged_ledger_diff::CoinBaseFeeTransfer { receiver_pk, fee } =
                    cb.inner().inner();
                CoinbaseFeeTransfer {
                    receiver_pk: PublicKey::from(receiver_pk),
                    fee: fee.inner().inner(),
                }
            })),
            staged_ledger_diff::CoinBase::CoinbaseAndFeeTransferViaCoinbase(x, y) => {
                Self::CoinbaseAndFeeTransferViaCoinbase(
                    x.map(|c| c.inner().inner().into()),
                    y.map(|c| c.inner().inner().into()),
                )
            }
        };
        let post_diff_coinbase = match precomputed_block
            .staged_ledger_post_diff()
            .map(|diff| diff.coinbase.inner())
        {
            None => None,
            Some(staged_ledger_diff::CoinBase::None) => Some(Self::None),
            Some(staged_ledger_diff::CoinBase::Coinbase(x)) => Some(Self::Coinbase(x.map(|cb| {
                let staged_ledger_diff::CoinBaseFeeTransfer { receiver_pk, fee } =
                    cb.inner().inner();
                CoinbaseFeeTransfer {
                    receiver_pk: PublicKey::from(receiver_pk),
                    fee: fee.inner().inner(),
                }
            }))),
            Some(staged_ledger_diff::CoinBase::CoinbaseAndFeeTransferViaCoinbase(x, y)) => {
                Some(Self::CoinbaseAndFeeTransferViaCoinbase(
                    x.map(|c| c.inner().inner().into()),
                    y.map(|c| c.inner().inner().into()),
                ))
            }
        };

        res.push(pre_diff_coinbase);
        if let Some(post_diff_coinbase) = post_diff_coinbase {
            res.push(post_diff_coinbase);
        }
        res
    }
}

impl Coinbase {
    pub fn amount(&self) -> u64 {
        if self.supercharge {
            2 * MAINNET_COINBASE_REWARD
        } else {
            MAINNET_COINBASE_REWARD
        }
    }

    pub fn from_precomputed(block: &PrecomputedBlock) -> Self {
        let kind = CoinbaseKind::from_precomputed(block);
        let kind = if kind.len() < 2 {
            kind[0].clone()
        } else {
            kind[1].clone()
        };
        Self {
            kind,
            receiver: block.coinbase_receiver(),
            receiver_balance: block.coinbase_receiver_balance(),
            is_new_account: block.accounts_created().1.is_some(),
            supercharge: block.consensus_state().supercharge_coinbase,
        }
    }

    // For fee_transfer_via_coinbase, remove the original fee_trasnfer for SNARK
    // work
    pub fn fee_transfer(&self) -> Vec<PaymentDiff> {
        self.get_transfers()
            .into_iter()
            .flat_map(|transfer| self.create_payment_diffs(transfer))
            .collect()
    }

    fn create_payment_diffs(&self, transfer: &CoinbaseFeeTransfer) -> Vec<PaymentDiff> {
        vec![
            PaymentDiff {
                public_key: transfer.receiver_pk.clone(),
                amount: transfer.fee.into(),
                update_type: UpdateType::Credit,
            },
            PaymentDiff {
                public_key: self.receiver.clone(),
                amount: transfer.fee.into(),
                update_type: UpdateType::Debit(None),
            },
        ]
    }

    fn get_transfers(&self) -> Vec<&CoinbaseFeeTransfer> {
        let transfers = match &self.kind {
            CoinbaseKind::None => vec![],
            CoinbaseKind::Coinbase(coinbase) => vec![coinbase.as_ref()],
            CoinbaseKind::CoinbaseAndFeeTransferViaCoinbase(
                coinbase,
                fee_transfer_via_coinbase,
            ) => {
                vec![coinbase.as_ref(), fee_transfer_via_coinbase.as_ref()]
            }
        };
        transfers.into_iter().flatten().collect::<Vec<_>>()
    }

    pub fn is_coinbase_applied(&self) -> bool {
        !matches!(self.kind, CoinbaseKind::None)
    }

    pub fn has_fee_transfer(&self) -> bool {
        !self.get_transfers().is_empty()
    }

    // only apply if "coinbase" =/= [ "Zero" ]
    pub fn as_account_diff(self) -> Vec<AccountDiff> {
        let mut res = vec![];
        if self.is_coinbase_applied() {
            res.append(&mut AccountDiff::from_coinbase(self));
        }
        res
    }

    pub fn as_internal_cmd(&self) -> InternalCommand {
        InternalCommand::Coinbase {
            receiver: self.receiver.clone(),
            amount: if self.supercharge {
                2 * MAINNET_COINBASE_REWARD
            } else {
                MAINNET_COINBASE_REWARD
            },
        }
    }
}

#[cfg(test)]
mod coinbase_tests {
    use super::*;

    fn sample_public_key() -> PublicKey {
        PublicKey::default()
    }

    #[test]
    fn test_coinbase_fee_transfer() {
        let transfer = CoinbaseFeeTransfer {
            receiver_pk: sample_public_key(),
            fee: 100,
        };

        let coinbase = Coinbase {
            kind: CoinbaseKind::Coinbase(Some(transfer.clone())),
            receiver: sample_public_key(),
            supercharge: false,
            is_new_account: false,
            receiver_balance: Some(0),
        };

        let payment_diffs = coinbase.fee_transfer();

        assert_eq!(payment_diffs.len(), 2);
        assert_eq!(payment_diffs[0].public_key, transfer.receiver_pk);
        assert_eq!(payment_diffs[0].amount, transfer.fee.into());
        assert_eq!(payment_diffs[0].update_type, UpdateType::Credit);
        assert_eq!(payment_diffs[1].public_key, coinbase.receiver);
        assert_eq!(payment_diffs[1].amount, transfer.fee.into());
        assert_eq!(payment_diffs[1].update_type, UpdateType::Debit(None));
    }

    #[test]
    fn test_coinbase_is_coinbase_applied() {
        let coinbase = Coinbase {
            kind: CoinbaseKind::None,
            receiver: sample_public_key(),
            supercharge: false,
            is_new_account: false,
            receiver_balance: Some(0),
        };

        assert!(!coinbase.is_coinbase_applied());

        let coinbase = Coinbase {
            kind: CoinbaseKind::Coinbase(None),
            ..coinbase
        };

        assert!(coinbase.is_coinbase_applied());
    }

    #[test]
    fn test_coinbase_has_fee_transfer() {
        let coinbase = Coinbase {
            kind: CoinbaseKind::None,
            receiver: sample_public_key(),
            supercharge: false,
            is_new_account: false,
            receiver_balance: Some(0),
        };

        assert!(!coinbase.has_fee_transfer());

        let coinbase = Coinbase {
            kind: CoinbaseKind::Coinbase(Some(CoinbaseFeeTransfer {
                receiver_pk: sample_public_key(),
                fee: 100,
            })),
            ..coinbase
        };

        assert!(coinbase.has_fee_transfer());
    }
}
