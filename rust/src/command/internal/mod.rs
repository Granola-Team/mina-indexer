pub mod store;

use crate::{
    block::{precomputed::PrecomputedBlock, BlockHash},
    ledger::{coinbase::Coinbase, diff::account::*, public_key::PublicKey},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum InternalCommandKind {
    Coinbase,

    #[serde(rename = "Fee_transfer")]
    FeeTransfer,

    #[serde(rename = "Fee_transfer_via_coinbase")]
    FeeTransferViaCoinbase,
}

/// This type is our internal representation of internal commands.
/// We use these internal commands for ledger calculations.
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

/// This type is used to get internal commands with metadata from the store.
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

/// This type is used to store internal commands in the store.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(untagged)]
pub enum DbInternalCommand {
    Coinbase { amount: u64, receiver: PublicKey },
    FeeTransfer { amount: u64, receiver: PublicKey },
    FeeTransferViaCoinbase { amount: u64, receiver: PublicKey },
}

impl InternalCommand {
    /// Compute the internal commands for the given precomputed block
    ///
    /// See [crate::ledger::diff::LedgerDiff::from_precomputed]
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
    pub fn from_internal_cmd(
        cmd: InternalCommand,
        state_hash: BlockHash,
        block_height: u32,
        date_time: i64,
    ) -> Self {
        match cmd {
            InternalCommand::Coinbase { receiver, amount } => Self::Coinbase {
                amount,
                receiver,
                date_time,
                state_hash,
                block_height,
                kind: InternalCommandKind::Coinbase,
            },
            InternalCommand::FeeTransfer {
                sender,
                receiver,
                amount,
            } => Self::FeeTransfer {
                amount,
                sender,
                receiver,
                date_time,
                state_hash,
                block_height,
                kind: InternalCommandKind::FeeTransfer,
            },
            InternalCommand::FeeTransferViaCoinbase {
                sender,
                receiver,
                amount,
            } => Self::FeeTransfer {
                amount,
                sender,
                receiver,
                date_time,
                state_hash,
                block_height,
                kind: InternalCommandKind::FeeTransferViaCoinbase,
            },
        }
    }

    pub fn from_precomputed(block: &PrecomputedBlock) -> Vec<Self> {
        InternalCommand::from_precomputed(block)
            .iter()
            .map(|cmd| {
                Self::from_internal_cmd(
                    cmd.clone(),
                    block.state_hash(),
                    block.blockchain_length(),
                    block.timestamp() as i64,
                )
            })
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

    pub fn recipient(&self) -> PublicKey {
        use InternalCommandWithData::*;
        match self {
            Coinbase { receiver, .. } | FeeTransfer { receiver, .. } => receiver.clone(),
        }
    }

    pub fn kind(&self) -> u8 {
        use InternalCommandWithData::*;
        match self {
            Coinbase { .. } => 1,
            FeeTransfer { .. } => 0,
        }
    }
}

impl std::fmt::Display for InternalCommandKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            InternalCommandKind::Coinbase => write!(f, "Coinbase"),
            InternalCommandKind::FeeTransfer => write!(f, "Fee_transfer"),
            InternalCommandKind::FeeTransferViaCoinbase => write!(f, "Fee_transfer_via_coinbase"),
        }
    }
}

impl DbInternalCommand {
    pub fn from_precomputed(block: &PrecomputedBlock) -> Vec<Self> {
        let internal_cmd_parts = InternalCommand::from_precomputed(block);
        let mut coinbase: Option<Self> = None;
        let mut fee_transfers = <HashMap<PublicKey, Self>>::new();
        let mut fee_transfers_via_coinbase = <HashMap<PublicKey, Self>>::new();
        for cmd in internal_cmd_parts {
            match cmd {
                InternalCommand::Coinbase { .. } => {
                    coinbase = Some(cmd.into());
                }
                InternalCommand::FeeTransfer {
                    ref receiver,
                    amount: amt,
                    ..
                } => match fee_transfers.get_mut(receiver) {
                    Some(Self::FeeTransfer { amount, .. }) => {
                        *amount += amt;
                    }
                    None => {
                        fee_transfers.insert(receiver.clone(), cmd.into());
                    }
                    _ => unimplemented!(),
                },
                InternalCommand::FeeTransferViaCoinbase {
                    ref sender,
                    amount: amt,
                    ..
                } => match fee_transfers_via_coinbase.get_mut(sender) {
                    Some(Self::FeeTransferViaCoinbase { amount, .. }) => {
                        *amount += amt;
                    }
                    None => {
                        fee_transfers_via_coinbase.insert(sender.clone(), cmd.into());
                    }
                    _ => unimplemented!(),
                },
            }
        }

        // coinbase
        let mut internal_commands = vec![];
        if let Some(cb) = coinbase {
            internal_commands.push(cb);
        }

        // fee transfers via coinbase
        let mut ftvc = fee_transfers_via_coinbase.into_values().collect::<Vec<_>>();
        ftvc.sort();
        internal_commands.append(&mut ftvc);

        // fee transfers
        let mut ft = fee_transfers.into_values().collect::<Vec<_>>();
        ft.sort();
        internal_commands.append(&mut ft);
        internal_commands
    }
}

impl From<InternalCommand> for DbInternalCommand {
    fn from(value: InternalCommand) -> Self {
        use InternalCommand::*;
        match value {
            Coinbase { receiver, amount } => Self::Coinbase { receiver, amount },
            FeeTransfer {
                receiver,
                amount,
                sender: _,
            } => Self::FeeTransfer { receiver, amount },
            FeeTransferViaCoinbase {
                receiver,
                amount,
                sender: _,
            } => Self::FeeTransferViaCoinbase { receiver, amount },
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
            .map(|cmd| {
                InternalCommandWithData::from_internal_cmd(
                    cmd,
                    block.state_hash(),
                    block.blockchain_length(),
                    block.timestamp() as i64,
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
