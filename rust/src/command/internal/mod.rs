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
        let mut all_account_diff_fees: Vec<Vec<AccountDiff>> = AccountDiff::from_block_fees(block);

        // replace Fee_transfer with Fee_transfer_via_coinbase, if any
        let coinbase = Coinbase::from_precomputed(block);
        if coinbase.has_fee_transfer() {
            coinbase.account_diffs_coinbase_mut(&mut all_account_diff_fees);
        }

        let mut internal_cmds: Vec<Self> = Vec::new();
        for account_diff_pairs in all_account_diff_fees {
            if let [credit, debit] = &account_diff_pairs[..] {
                if debit.amount() + credit.amount() != 0 {
                    panic!(
                        "Credit/Debit pairs do not sum to zero.\nDebit: {:#?}\nCredit: {:#?}\nBlock Height: {:#?}",
                        debit, credit, block.blockchain_length()
                    )
                }
                match (credit, debit) {
                    (AccountDiff::CreateAccount(_), AccountDiff::CreateAccount(_)) => {
                        println!(
                            "AccountDiff::CreateAccount credit and debit pairs are present but unhandled"
                        );
                    }
                    (
                        AccountDiff::FeeTransfer(fee_transfer_receiver),
                        AccountDiff::FeeTransfer(fee_transfer_sender),
                    ) => {
                        if let Some(Self::FeeTransfer { amount, .. }) =
                            internal_cmds.iter_mut().find(|cmd| match cmd {
                                Self::FeeTransfer {
                                    sender, receiver, ..
                                } => {
                                    sender.0 == fee_transfer_sender.public_key.0
                                        && receiver.0 == fee_transfer_receiver.public_key.0
                                }
                                _ => false,
                            })
                        {
                            *amount += fee_transfer_receiver.amount.0;
                        } else {
                            internal_cmds.push(Self::FeeTransfer {
                                sender: fee_transfer_sender.public_key.clone(),
                                receiver: fee_transfer_receiver.public_key.clone(),
                                amount: fee_transfer_sender.amount.0,
                            })
                        }
                    }
                    (
                        AccountDiff::FeeTransferViaCoinbase(fee_transfer_receiver),
                        AccountDiff::FeeTransferViaCoinbase(fee_transfer_sender),
                    ) => internal_cmds.push(Self::FeeTransferViaCoinbase {
                        sender: fee_transfer_sender.public_key.clone(),
                        receiver: fee_transfer_receiver.public_key.clone(),
                        amount: fee_transfer_sender.amount.0,
                    }),
                    (_, _) => panic!(
                        "Unrecognized credit/debit comination. Block: {:#?}, hash: {:#?}",
                        block.blockchain_length(),
                        block.state_hash(),
                    ),
                }
            } else if let [imbalanced_diff] = &account_diff_pairs[..] {
                match imbalanced_diff {
                    AccountDiff::Coinbase(coinbase) => internal_cmds.push(Self::Coinbase {
                        receiver: coinbase.public_key.clone(),
                        amount: coinbase.amount.0,
                    }),
                    _ => {
                        panic!(
                            "Unmatched AccountDiff::{:#?}. (Block: {:#?}, hash: {:#?})",
                            imbalanced_diff,
                            block.blockchain_length(),
                            block.state_hash(),
                        );
                    }
                };
            } else {
                panic!(
                    "Unrecognized accounting arrangement. {:#?}",
                    &account_diff_pairs[..]
                );
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
