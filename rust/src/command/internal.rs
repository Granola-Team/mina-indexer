use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    ledger::{coinbase::Coinbase, diff::account::*, public_key::PublicKey},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum InternalCommandKind {
    Coinbase,
    #[serde(rename = "Fee_transfer")]
    FeeTransfer,
    #[serde(rename = "Fee_transfer_via_coinbase")]
    FeeTransferViaCoinbase,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum InternalCommand {
    Coinbase {
        receiver: PublicKey,
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum InternalCommandWithData {
    Coinbase {
        receiver: PublicKey,
        amount: u64,
        state_hash: BlockHash,
        kind: InternalCommandKind,
    },
    // Fee_transfer or Fee_transfer_via_coinbase
    FeeTransfer {
        sender: PublicKey,
        receiver: PublicKey,
        amount: u64,
        state_hash: BlockHash,
        kind: InternalCommandKind,
    },
}

impl InternalCommand {
    /// Compute the internal commands for the given precomputed block
    ///
    /// See [LedgerDiff::from_precomputed](../ledger/diff/mod.rs#L21)
    pub fn from_precomputed(precomputed_block: &PrecomputedBlock) -> Vec<Self> {
        let mut account_diff_fees = AccountDiff::from_block_fees(precomputed_block);

        // replace Fee_transfer with Fee_transfer_via_coinbase, if any
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
                    receiver: c.public_key.clone(),
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

impl InternalCommandWithData {
    pub fn from_internal_cmd(cmd: InternalCommand, state_hash: &BlockHash) -> Self {
        match cmd {
            InternalCommand::Coinbase { receiver, amount } => Self::Coinbase {
                receiver,
                amount,
                state_hash: state_hash.clone(),
                kind: InternalCommandKind::Coinbase,
            },
            InternalCommand::FeeTransfer {
                sender,
                receiver,
                amount,
            } => Self::FeeTransfer {
                sender,
                receiver,
                amount,
                state_hash: state_hash.clone(),
                kind: InternalCommandKind::FeeTransfer,
            },
            InternalCommand::FeeTransferViaCoinbase {
                sender,
                receiver,
                amount,
            } => Self::FeeTransfer {
                sender,
                receiver,
                amount,
                state_hash: state_hash.clone(),
                kind: InternalCommandKind::FeeTransferViaCoinbase,
            },
        }
    }

    pub fn public_keys(&self) -> Vec<PublicKey> {
        match self {
            Self::Coinbase { receiver, .. } => vec![receiver.clone()],
            Self::FeeTransfer {
                sender, receiver, ..
            } => vec![sender.clone(), receiver.clone()],
        }
    }

    pub fn contains_pk(&self, pk: &PublicKey) -> bool {
        match self {
            Self::Coinbase { receiver, .. } => pk == receiver,
            Self::FeeTransfer {
                sender, receiver, ..
            } => pk == sender || pk == receiver,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
                    receiver: "B62qs2YyNuo1LbNo5sbhPByDDAB7NZiejFM6H1ctND5ui7wH4PWa7qm".into(),
                    amount: 720000000000
                },
                InternalCommand::FeeTransfer {
                    sender: "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy".into(),
                    receiver: "B62qs2YyNuo1LbNo5sbhPByDDAB7NZiejFM6H1ctND5ui7wH4PWa7qm".into(),
                    amount: 20000000
                }
            ]
        );

        let cmds: Vec<InternalCommandWithData> = internal_cmds
            .into_iter()
            .map(|cmd| {
                InternalCommandWithData::from_internal_cmd(
                    cmd,
                    &"3NLMeYAFXxsmhSFtLHFxdtjGcfHTVFmBmBF8uTJvP4Ve5yEmxYeA".into(),
                )
            })
            .collect();
        assert_eq!(
            cmds,
            vec![
                InternalCommandWithData::Coinbase {
                    receiver: "B62qs2YyNuo1LbNo5sbhPByDDAB7NZiejFM6H1ctND5ui7wH4PWa7qm".into(),
                    amount: 720000000000,
                    state_hash: "3NLMeYAFXxsmhSFtLHFxdtjGcfHTVFmBmBF8uTJvP4Ve5yEmxYeA".into(),
                    kind: InternalCommandKind::Coinbase
                },
                InternalCommandWithData::FeeTransfer {
                    sender: "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy".into(),
                    receiver: "B62qs2YyNuo1LbNo5sbhPByDDAB7NZiejFM6H1ctND5ui7wH4PWa7qm".into(),
                    amount: 20000000,
                    state_hash: "3NLMeYAFXxsmhSFtLHFxdtjGcfHTVFmBmBF8uTJvP4Ve5yEmxYeA".into(),
                    kind: InternalCommandKind::FeeTransfer,
                }
            ]
        );

        Ok(())
    }
}
