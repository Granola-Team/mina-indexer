//! Indexer internal coinbase representation

use crate::{
    block::precomputed::PrecomputedBlock,
    command::internal::InternalCommand,
    constants::*,
    ledger::{
        diff::account::{AccountDiff, PaymentDiff, UpdateType},
        PublicKey,
    },
    mina_blocks::v2,
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

#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub enum CoinbaseKind {
    Zero,
    One(Option<CoinbaseFeeTransfer>),
    Two(Option<CoinbaseFeeTransfer>, Option<CoinbaseFeeTransfer>),
}

#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct CoinbaseFeeTransfer {
    pub receiver_pk: PublicKey,
    pub fee: u64,
}

///////////
// impls //
///////////

impl CoinbaseKind {
    pub fn from_precomputed(block: &PrecomputedBlock) -> Self {
        let mut kind = vec![block.pre_diff_coinbase()];

        if let Some(post_diff_coinbase) = block.post_diff_coinbase() {
            kind.push(post_diff_coinbase);
        }

        kind.into_iter().max().expect("max coinbase")
    }
}

impl Coinbase {
    pub fn amount(&self) -> u64 {
        if matches!(self.kind, CoinbaseKind::Zero) {
            0
        } else if !self.supercharge {
            MAINNET_COINBASE_REWARD
        } else {
            2 * MAINNET_COINBASE_REWARD
        }
    }

    pub fn from_precomputed(block: &PrecomputedBlock) -> Self {
        Self {
            kind: CoinbaseKind::from_precomputed(block),
            receiver: block.coinbase_receiver(),
            receiver_balance: block.coinbase_receiver_balance(),
            is_new_account: block.accounts_created().1.is_some(),
            supercharge: block.supercharge_coinbase(),
        }
    }

    // For fee_transfer_via_coinbase, remove the original fee_trasnfer for SNARK
    // work
    pub fn fee_transfer(&self) -> Vec<Vec<PaymentDiff>> {
        self.get_transfers()
            .into_iter()
            .map(|transfer| self.create_payment_diffs(transfer))
            .collect()
    }

    pub fn account_diffs_coinbase_mut(&self, account_diffs: &mut [Vec<AccountDiff>]) {
        let fee_transfer = self.fee_transfer();

        if let Some(fee_transfer_pair) = account_diffs.iter_mut().find(|pair| {
            matches!(pair.as_slice(),
                [AccountDiff::FeeTransfer(fee_transfer_credit), AccountDiff::FeeTransfer(fee_transfer_debit)]
                if &fee_transfer[0][0] == fee_transfer_credit
                && &fee_transfer[0][1] == fee_transfer_debit)
        }) {
            fee_transfer_pair[0] =
                AccountDiff::FeeTransferViaCoinbase(fee_transfer[0][0].clone());
            fee_transfer_pair[1] =
                AccountDiff::FeeTransferViaCoinbase(fee_transfer[0][1].clone());
        }
    }

    fn create_payment_diffs(&self, transfer: &CoinbaseFeeTransfer) -> Vec<PaymentDiff> {
        vec![
            PaymentDiff {
                public_key: transfer.receiver_pk.clone(),
                amount: transfer.fee.into(),
                update_type: UpdateType::Credit,
                txn_hash: None,
                token: None, // always MINA
            },
            PaymentDiff {
                public_key: self.receiver.clone(),
                amount: transfer.fee.into(),
                update_type: UpdateType::Debit(None),
                txn_hash: None,
                token: None, // always MINA
            },
        ]
    }

    fn get_transfers(&self) -> Vec<&CoinbaseFeeTransfer> {
        let transfers = match &self.kind {
            CoinbaseKind::Zero => vec![],
            CoinbaseKind::One(coinbase) => vec![coinbase.as_ref()],
            CoinbaseKind::Two(coinbase, fee_transfer_via_coinbase) => {
                vec![coinbase.as_ref(), fee_transfer_via_coinbase.as_ref()]
            }
        };

        transfers.into_iter().flatten().collect::<Vec<_>>()
    }

    pub fn is_applied(&self) -> bool {
        !matches!(self.kind, CoinbaseKind::Zero)
    }

    pub fn has_fee_transfer(&self) -> bool {
        !self.get_transfers().is_empty()
    }

    // only apply if "coinbase" =/= [ "Zero" ]
    pub fn as_account_diff(self) -> Vec<Vec<AccountDiff>> {
        if self.is_applied() {
            return AccountDiff::from_coinbase(self);
        }

        vec![]
    }

    pub fn as_internal_cmd(&self) -> InternalCommand {
        InternalCommand::Coinbase {
            receiver: self.receiver.clone(),
            amount: self.amount(),
        }
    }
}

/////////////////
// conversions //
/////////////////

impl From<v2::staged_ledger_diff::Coinbase> for CoinbaseKind {
    fn from(value: v2::staged_ledger_diff::Coinbase) -> Self {
        match value {
            v2::staged_ledger_diff::Coinbase::Zero(_) => Self::Zero,
            v2::staged_ledger_diff::Coinbase::One(_, one) => {
                Self::One(one.map(|o| CoinbaseFeeTransfer {
                    receiver_pk: o.receiver_pk,
                    fee: o.fee.0,
                }))
            }
            v2::staged_ledger_diff::Coinbase::Two(_, two) => match two {
                None => Self::Two(None, None),
                Some((fst, snd)) => Self::Two(
                    Some(CoinbaseFeeTransfer {
                        receiver_pk: fst.receiver_pk,
                        fee: fst.fee.0,
                    }),
                    snd.map(|s| CoinbaseFeeTransfer {
                        receiver_pk: s.receiver_pk,
                        fee: s.fee.0,
                    }),
                ),
            },
        }
    }
}

impl From<staged_ledger_diff::CoinBaseFeeTransfer> for CoinbaseFeeTransfer {
    fn from(value: staged_ledger_diff::CoinBaseFeeTransfer) -> Self {
        Self {
            receiver_pk: PublicKey::from(value.receiver_pk),
            fee: value.fee.inner().inner(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::precomputed::PcbVersion;

    #[test]
    fn test_coinbase_fee_transfer() {
        let transfer = CoinbaseFeeTransfer {
            receiver_pk: PublicKey::default(),
            fee: 100,
        };
        let coinbase = Coinbase {
            kind: CoinbaseKind::One(Some(transfer.clone())),
            receiver: PublicKey::default(),
            supercharge: false,
            is_new_account: false,
            receiver_balance: Some(0),
        };
        let payment_diffs = coinbase.fee_transfer();

        assert_eq!(payment_diffs[0].len(), 2);

        if let [credit, debit] = &payment_diffs[0][..] {
            assert_eq!(credit.public_key, transfer.receiver_pk);
            assert_eq!(credit.amount, transfer.fee.into());
            assert_eq!(credit.update_type, UpdateType::Credit);
            assert_eq!(debit.public_key, coinbase.receiver);
            assert_eq!(debit.amount, transfer.fee.into());
            assert_eq!(debit.update_type, UpdateType::Debit(None));
        } else {
            panic!("Expected debit/credit pair")
        }
    }

    #[test]
    fn test_coinbase_is_coinbase_applied() {
        let coinbase = Coinbase {
            kind: CoinbaseKind::Zero,
            receiver: PublicKey::default(),
            supercharge: false,
            is_new_account: false,
            receiver_balance: Some(0),
        };
        assert!(!coinbase.is_applied());

        let coinbase = Coinbase {
            kind: CoinbaseKind::One(None),
            ..coinbase
        };
        assert!(coinbase.is_applied());
    }

    #[test]
    fn test_account_diffs_coinbase_mut() {
        let transfer = CoinbaseFeeTransfer {
            receiver_pk: PublicKey::default(),
            fee: 100,
        };
        let coinbase = Coinbase {
            kind: CoinbaseKind::One(Some(transfer.clone())),
            receiver: PublicKey::default(),
            supercharge: false,
            is_new_account: false,
            receiver_balance: Some(0),
        };
        let fee_transfer_payment_diffs = coinbase.fee_transfer();
        let mut account_diffs = vec![vec![
            AccountDiff::FeeTransfer(fee_transfer_payment_diffs[0][0].clone()),
            AccountDiff::FeeTransfer(fee_transfer_payment_diffs[0][1].clone()),
        ]];

        coinbase.account_diffs_coinbase_mut(&mut account_diffs);
        assert_eq!(
            account_diffs[0],
            vec![
                AccountDiff::FeeTransferViaCoinbase(fee_transfer_payment_diffs[0][0].clone()),
                AccountDiff::FeeTransferViaCoinbase(fee_transfer_payment_diffs[0][1].clone())
            ]
        );
    }

    #[test]
    fn test_coinbase_has_fee_transfer() {
        let coinbase = Coinbase {
            kind: CoinbaseKind::Zero,
            receiver: PublicKey::default(),
            supercharge: false,
            is_new_account: false,
            receiver_balance: Some(0),
        };
        assert!(!coinbase.has_fee_transfer());

        let coinbase = Coinbase {
            kind: CoinbaseKind::One(Some(CoinbaseFeeTransfer {
                receiver_pk: PublicKey::default(),
                fee: 100,
            })),
            ..coinbase
        };
        assert!(coinbase.has_fee_transfer());
    }

    #[test]
    fn coinbase_from_precomputed_v1() -> anyhow::Result<()> {
        let path = std::path::PathBuf::from("./tests/data/misc_blocks/mainnet-278424-3NLbUZF8568pK56NJuSpCkfLTQTKpoiNiruju1Hpr6qpoAbuN9Yr.json");
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V1)?;
        let expect = Coinbase {
            kind: CoinbaseKind::One(None),
            receiver: PublicKey::from("B62qjHdYUPTHQkwDWUbDYscteT2LFj3ro1vz9fnxMyHTACe6C2fLbSd"),
            supercharge: false,
            is_new_account: false,
            receiver_balance: Some(16790466359034),
        };

        assert_eq!(Coinbase::from_precomputed(&block), expect);
        Ok(())
    }

    #[test]
    fn coinbase_from_precomputed_v2() -> anyhow::Result<()> {
        let path = std::path::PathBuf::from("./tests/data/misc_blocks/mainnet-419989-3NKhZKc1HrEexmpvcbx4eqAtrsbwmfLXcjukF9CJ8Y2y7FEjFWg5.json");
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;
        let expect = Coinbase {
            kind: CoinbaseKind::One(None),
            receiver: PublicKey::from("B62qospDjUj43x2yMKiNehojWWRUsE1wpdUDVpfxH8V3n5Y1QgJKFfw"),
            supercharge: false,
            is_new_account: false,
            receiver_balance: None,
        };

        assert_eq!(Coinbase::from_precomputed(&block), expect);
        Ok(())
    }

    #[test]
    fn genesis_v1() -> anyhow::Result<()> {
        let path = std::path::PathBuf::from("./data/genesis_blocks/mainnet-1-3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ.json");
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V1)?;
        let coinbase = Coinbase::from_precomputed(&block);

        assert_eq!(
            coinbase,
            Coinbase {
                kind: CoinbaseKind::Zero,
                receiver: "B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg".into(),
                supercharge: true,
                is_new_account: false,
                receiver_balance: None,
            }
        );
        Ok(())
    }

    #[test]
    fn genesis_v2() -> anyhow::Result<()> {
        let path = std::path::PathBuf::from("./data/genesis_blocks/mainnet-359605-3NK4BpDSekaqsG6tx8Nse2zJchRft2JpnbvMiog55WCr5xJZaKeP.json");
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;
        let coinbase = Coinbase::from_precomputed(&block);

        assert_eq!(
            coinbase,
            Coinbase {
                kind: CoinbaseKind::Zero,
                receiver: "B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg".into(),
                supercharge: false,
                is_new_account: false,
                receiver_balance: None,
            }
        );

        assert!(!coinbase.is_applied());
        assert!(!coinbase.has_fee_transfer());

        Ok(())
    }
}
