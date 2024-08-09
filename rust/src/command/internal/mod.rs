pub mod store;

use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    ledger::{coinbase::Coinbase, diff::account::*, public_key::PublicKey},
};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum InternalCommandKind {
    Coinbase,
    #[serde(rename = "Fee_transfer")]
    FeeTransfer,
    #[serde(rename = "Fee_transfer_via_coinbase")]
    FeeTransferViaCoinbase,
}

impl fmt::Display for InternalCommandKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InternalCommandKind::Coinbase => write!(f, "Coinbase"),
            InternalCommandKind::FeeTransfer => write!(f, "Fee_transfer"),
            InternalCommandKind::FeeTransferViaCoinbase => write!(f, "Fee_transfer_via_coinbase"),
        }
    }
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
    // Fee_transfer or Fee_transfer_via_coinbase
    FeeTransfer {
        sender: PublicKey,
        receiver: PublicKey,
        amount: u64,
        state_hash: BlockHash,
        kind: InternalCommandKind,
        date_time: i64,
        block_height: u32,
    },
    Coinbase {
        receiver: PublicKey,
        amount: u64,
        state_hash: BlockHash,
        kind: InternalCommandKind,
        date_time: i64,
        block_height: u32,
    },
}

impl InternalCommand {
    /// Compute the internal commands for the given precomputed block
    ///
    /// See `LedgerDiff::from_precomputed`
    pub fn from_precomputed(block: &PrecomputedBlock) -> Vec<Self> {
        let mut account_diff_fees = AccountDiff::from_block_fees(block);

        // replace Fee_transfer with Fee_transfer_via_coinbase, if any
        let coinbase = Coinbase::from_precomputed(block);
        if coinbase.has_fee_transfer() {
            coinbase.fee_transfer().map(|fee_transfer| {
                let idx = account_diff_fees
                    .iter()
                    .enumerate()
                    .position(|(n, diff)| match diff {
                        AccountDiff::FeeTransfer(fee) => {
                            *fee == fee_transfer.credit
                                && match &account_diff_fees[n + 1] {
                                    AccountDiff::FeeTransfer(fee) => *fee == fee_transfer.debit,
                                    _ => false,
                                }
                        }
                        _ => false,
                    });

                idx.iter().for_each(|i| {
                    account_diff_fees[*i] =
                        AccountDiff::FeeTransferViaCoinbase(fee_transfer.credit.clone());
                    account_diff_fees[*i + 1] =
                        AccountDiff::FeeTransferViaCoinbase(fee_transfer.debit.clone());
                });
            });
        }

        let mut internal_cmds = vec![];
        for n in (0..account_diff_fees.len()).step_by(2) {
            match &account_diff_fees[n] {
                AccountDiff::FeeTransfer(f) => {
                    let (ic_sender, ic_receiver) = if f.update_type == UpdateType::Credit {
                        (account_diff_fees[n + 1].public_key(), f.public_key.clone())
                    } else {
                        (f.public_key.clone(), account_diff_fees[n + 1].public_key())
                    };

                    // aggregate
                    if let Some((idx, cmd)) =
                        internal_cmds
                            .iter()
                            .enumerate()
                            .find_map(|(m, cmd)| match cmd {
                                Self::FeeTransfer {
                                    sender,
                                    receiver,
                                    amount,
                                } => {
                                    if *sender == ic_sender && *receiver == ic_receiver {
                                        return Some((
                                            m,
                                            Self::FeeTransfer {
                                                sender: ic_sender.clone(),
                                                receiver: ic_receiver.clone(),
                                                amount: f.amount.0 + *amount,
                                            },
                                        ));
                                    }
                                    None
                                }
                                _ => None,
                            })
                    {
                        internal_cmds.push(cmd);
                        internal_cmds.swap_remove(idx);
                    } else {
                        internal_cmds.push(Self::FeeTransfer {
                            sender: ic_sender.clone(),
                            receiver: ic_receiver.clone(),
                            amount: f.amount.0,
                        });
                    }
                }
                AccountDiff::FeeTransferViaCoinbase(f) => {
                    let (sender, receiver) = if f.update_type == UpdateType::Credit {
                        (account_diff_fees[n + 1].public_key(), f.public_key.clone())
                    } else {
                        (f.public_key.clone(), account_diff_fees[n + 1].public_key())
                    };
                    internal_cmds.push(Self::FeeTransferViaCoinbase {
                        sender,
                        receiver,
                        amount: f.amount.0,
                    });
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
    pub fn from_internal_cmd(cmd: InternalCommand, block: &PrecomputedBlock) -> Self {
        match cmd {
            InternalCommand::Coinbase { receiver, amount } => Self::Coinbase {
                receiver,
                amount,
                state_hash: block.state_hash(),
                kind: InternalCommandKind::Coinbase,
                block_height: block.blockchain_length(),
                date_time: block.timestamp() as i64,
            },
            InternalCommand::FeeTransfer {
                sender,
                receiver,
                amount,
            } => Self::FeeTransfer {
                sender,
                receiver,
                amount,
                state_hash: block.state_hash(),
                kind: InternalCommandKind::FeeTransfer,
                block_height: block.blockchain_length(),
                date_time: block.timestamp() as i64,
            },
            InternalCommand::FeeTransferViaCoinbase {
                sender,
                receiver,
                amount,
            } => Self::FeeTransfer {
                sender,
                receiver,
                amount,
                state_hash: block.state_hash(),
                kind: InternalCommandKind::FeeTransferViaCoinbase,
                block_height: block.blockchain_length(),
                date_time: block.timestamp() as i64,
            },
        }
    }

    pub fn from_precomputed(block: &PrecomputedBlock) -> Vec<Self> {
        InternalCommand::from_precomputed(block)
            .iter()
            .map(|cmd| Self::from_internal_cmd(cmd.clone(), block))
            .collect()
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
    use crate::block::precomputed::{PcbVersion, PrecomputedBlock};

    #[test]
    fn from_precomputed() -> anyhow::Result<()> {
        let path = std::path::PathBuf::from("./tests/data/canonical_chain_discovery/contiguous/mainnet-11-3NLMeYAFXxsmhSFtLHFxdtjGcfHTVFmBmBF8uTJvP4Ve5yEmxYeA.json");
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V1)?;
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
            .map(|cmd| InternalCommandWithData::from_internal_cmd(cmd, &block))
            .collect();
        assert_eq!(
            cmds,
            vec![
                InternalCommandWithData::Coinbase {
                    receiver: "B62qs2YyNuo1LbNo5sbhPByDDAB7NZiejFM6H1ctND5ui7wH4PWa7qm".into(),
                    amount: 720000000000,
                    state_hash: "3NLMeYAFXxsmhSFtLHFxdtjGcfHTVFmBmBF8uTJvP4Ve5yEmxYeA".into(),
                    kind: InternalCommandKind::Coinbase,
                    block_height: block.blockchain_length(),
                    date_time: block.timestamp() as i64,
                },
                InternalCommandWithData::FeeTransfer {
                    sender: "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy".into(),
                    receiver: "B62qs2YyNuo1LbNo5sbhPByDDAB7NZiejFM6H1ctND5ui7wH4PWa7qm".into(),
                    amount: 20000000,
                    state_hash: "3NLMeYAFXxsmhSFtLHFxdtjGcfHTVFmBmBF8uTJvP4Ve5yEmxYeA".into(),
                    kind: InternalCommandKind::FeeTransfer,
                    block_height: block.blockchain_length(),
                    date_time: block.timestamp() as i64,
                }
            ]
        );

        Ok(())
    }
}
