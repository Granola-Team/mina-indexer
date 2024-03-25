use crate::{
    block::precomputed::PrecomputedBlock,
    ledger::{coinbase::Coinbase, diff::account::*, public_key::PublicKey},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum InternalCommand {
    Coinbase {
        pk: PublicKey,
        amount: u64,
    },
    FeeTransfer {
        sender: PublicKey,
        receiver: PublicKey,
        amount: u64,
    },
    FeeTransferViaCoinbase {
        sender: PublicKey,
        receiver: PublicKey,
        amount: u64,
    },
}

impl InternalCommand {
    /// Compute the internal commands for the given precomputed block
    ///
    /// See [LedgerDiff::from_precomputed](../ledger/diff/mod.rs#L21)
    pub fn from_precomputed(precomputed_block: &PrecomputedBlock) -> Vec<Self> {
        let mut account_diff_fees = AccountDiff::from_block_fees(precomputed_block);

        // replace fee_transfer with fee_transfer_via_coinbase, if any
        let coinbase = Coinbase::from_precomputed(precomputed_block);
        if coinbase.has_fee_transfer() {
            let fee_transfer = coinbase.fee_transfer();
            let idx = account_diff_fees
                .iter()
                .enumerate()
                .position(|(n, diff)| match diff {
                    AccountDiff::FeeTransfer(fee) => {
                        *fee == fee_transfer[0]
                            && match &account_diff_fees[n + 1] {
                                AccountDiff::FeeTransfer(fee) => *fee == fee_transfer[1],
                                _ => false,
                            }
                    }
                    _ => false,
                });
            idx.iter().for_each(|i| {
                account_diff_fees[*i] =
                    AccountDiff::FeeTransferViaCoinbase(fee_transfer[0].clone());
                account_diff_fees[*i + 1] =
                    AccountDiff::FeeTransferViaCoinbase(fee_transfer[1].clone());
            });
        }

        let mut internal_cmds = vec![];
        for n in (0..account_diff_fees.len()).step_by(2) {
            match &account_diff_fees[n] {
                AccountDiff::FeeTransfer(f) => {
                    if f.update_type == UpdateType::Credit {
                        internal_cmds.push(Self::FeeTransfer {
                            sender: account_diff_fees[n + 1].public_key(),
                            receiver: f.public_key.clone(),
                            amount: f.amount.0,
                        })
                    } else {
                        internal_cmds.push(Self::FeeTransfer {
                            sender: f.public_key.clone(),
                            receiver: account_diff_fees[n + 1].public_key(),
                            amount: f.amount.0,
                        })
                    }
                }
                AccountDiff::FeeTransferViaCoinbase(f) => {
                    if f.update_type == UpdateType::Credit {
                        internal_cmds.push(Self::FeeTransferViaCoinbase {
                            sender: account_diff_fees[n + 1].public_key(),
                            receiver: f.public_key.clone(),
                            amount: f.amount.0,
                        })
                    } else {
                        internal_cmds.push(Self::FeeTransferViaCoinbase {
                            sender: f.public_key.clone(),
                            receiver: account_diff_fees[n + 1].public_key(),
                            amount: f.amount.0,
                        })
                    }
                }
                AccountDiff::Coinbase(c) => internal_cmds.push(Self::Coinbase {
                    pk: c.public_key.clone(),
                    amount: c.amount.0,
                }),
                _ => (),
            }
        }

        if coinbase.is_coinbase_applied() {
            internal_cmds.insert(0, coinbase.as_internal_cmd());
        }
        internal_cmds
    }
}

#[cfg(test)]
mod tests {
    use super::InternalCommand;
    use crate::block::precomputed::PrecomputedBlock;

    #[test]
    fn from_precomputed() -> anyhow::Result<()> {
        let path = std::path::PathBuf::from("./tests/initial-blocks/mainnet-11-3NLMeYAFXxsmhSFtLHFxdtjGcfHTVFmBmBF8uTJvP4Ve5yEmxYeA.json");
        let block = PrecomputedBlock::parse_file(&path)?;
        let internal_cmds = InternalCommand::from_precomputed(&block);

        assert_eq!(
            internal_cmds,
            vec![
                InternalCommand::Coinbase {
                    pk: "B62qs2YyNuo1LbNo5sbhPByDDAB7NZiejFM6H1ctND5ui7wH4PWa7qm".into(),
                    amount: 720000000000
                },
                InternalCommand::FeeTransfer {
                    sender: "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy".into(),
                    receiver: "B62qs2YyNuo1LbNo5sbhPByDDAB7NZiejFM6H1ctND5ui7wH4PWa7qm".into(),
                    amount: 20000000
                }
            ]
        );
        Ok(())
    }
}
