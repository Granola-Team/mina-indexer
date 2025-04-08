pub mod store;

use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::precomputed::PrecomputedBlock,
    ledger::{coinbase::Coinbase, diff::account::*},
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

        let mut internal_cmds = vec![];
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

        if coinbase.is_applied() {
            internal_cmds.insert(0, coinbase.as_internal_cmd());
        }

        internal_cmds
    }
}

/// This type is used to store internal commands in the store.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(untagged)]
pub enum DbInternalCommand {
    Coinbase { amount: u64, receiver: PublicKey },
    FeeTransfer { amount: u64, receiver: PublicKey },
    FeeTransferViaCoinbase { amount: u64, receiver: PublicKey },
}

/// This type is used to get internal commands with metadata from the store.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum DbInternalCommandWithData {
    // Fee_transfer or Fee_transfer_via_coinbase
    FeeTransfer {
        receiver: PublicKey,
        amount: u64,
        state_hash: StateHash,
        kind: InternalCommandKind,
        date_time: i64,
        block_height: u32,
    },
    Coinbase {
        receiver: PublicKey,
        amount: u64,
        state_hash: StateHash,
        kind: InternalCommandKind,
        date_time: i64,
        block_height: u32,
    },
}

impl DbInternalCommandWithData {
    pub fn from_internal_cmd(
        cmd: DbInternalCommand,
        state_hash: StateHash,
        block_height: u32,
        date_time: i64,
    ) -> Self {
        match cmd {
            DbInternalCommand::Coinbase { receiver, amount } => Self::Coinbase {
                amount,
                receiver,
                date_time,
                state_hash,
                block_height,
                kind: InternalCommandKind::Coinbase,
            },
            DbInternalCommand::FeeTransfer { receiver, amount } => Self::FeeTransfer {
                amount,
                receiver,
                date_time,
                state_hash,
                block_height,
                kind: InternalCommandKind::FeeTransfer,
            },
            DbInternalCommand::FeeTransferViaCoinbase { receiver, amount } => Self::FeeTransfer {
                amount,
                receiver,
                date_time,
                state_hash,
                block_height,
                kind: InternalCommandKind::FeeTransferViaCoinbase,
            },
        }
    }

    pub fn from_precomputed(block: &PrecomputedBlock) -> Vec<Self> {
        DbInternalCommand::from_precomputed(block)
            .into_iter()
            .map(|cmd| {
                Self::from_internal_cmd(
                    cmd,
                    block.state_hash(),
                    block.blockchain_length(),
                    block.timestamp() as i64,
                )
            })
            .collect()
    }

    pub fn public_keys(&self) -> PublicKey {
        match self {
            Self::Coinbase { receiver, .. } | Self::FeeTransfer { receiver, .. } => {
                receiver.clone()
            }
        }
    }

    pub fn contains_pk(&self, pk: &PublicKey) -> bool {
        match self {
            Self::Coinbase { receiver, .. } | Self::FeeTransfer { receiver, .. } => pk == receiver,
        }
    }

    pub fn recipient(&self) -> PublicKey {
        match self {
            DbInternalCommandWithData::Coinbase { receiver, .. }
            | DbInternalCommandWithData::FeeTransfer { receiver, .. } => receiver.clone(),
        }
    }

    pub fn kind(&self) -> u8 {
        match self {
            DbInternalCommandWithData::FeeTransfer { .. } => 0,
            DbInternalCommandWithData::Coinbase { .. } => 1,
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

        // fee transfers
        let mut ft = fee_transfers.into_values().collect::<Vec<_>>();
        ft.sort();

        internal_commands.append(&mut ftvc);
        internal_commands.append(&mut ft);

        internal_commands
    }
}

/////////////////
// conversions //
/////////////////

impl From<InternalCommand> for DbInternalCommand {
    fn from(value: InternalCommand) -> Self {
        match value {
            InternalCommand::Coinbase { receiver, amount } => Self::Coinbase { receiver, amount },
            InternalCommand::FeeTransfer {
                receiver,
                amount,
                sender: _,
            } => Self::FeeTransfer { receiver, amount },
            InternalCommand::FeeTransferViaCoinbase {
                receiver,
                amount,
                sender: _,
            } => Self::FeeTransferViaCoinbase { receiver, amount },
        }
    }
}

/////////////
// dispaly //
/////////////

impl std::fmt::Display for InternalCommandKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            InternalCommandKind::Coinbase => write!(f, "Coinbase"),
            InternalCommandKind::FeeTransfer => write!(f, "Fee_transfer"),
            InternalCommandKind::FeeTransferViaCoinbase => write!(f, "Fee_transfer_via_coinbase"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::precomputed::{PcbVersion, PrecomputedBlock};

    #[test]
    fn from_precomputed_v1() -> anyhow::Result<()> {
        let path = std::path::PathBuf::from("./tests/data/canonical_chain_discovery/contiguous/mainnet-11-3NLMeYAFXxsmhSFtLHFxdtjGcfHTVFmBmBF8uTJvP4Ve5yEmxYeA.json");
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V1)?;

        assert_eq!(
            InternalCommand::from_precomputed(&block),
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

        let internal_cmds = DbInternalCommand::from_precomputed(&block);
        assert_eq!(
            internal_cmds,
            vec![
                DbInternalCommand::Coinbase {
                    receiver: "B62qs2YyNuo1LbNo5sbhPByDDAB7NZiejFM6H1ctND5ui7wH4PWa7qm".into(),
                    amount: 720000000000
                },
                DbInternalCommand::FeeTransfer {
                    receiver: "B62qs2YyNuo1LbNo5sbhPByDDAB7NZiejFM6H1ctND5ui7wH4PWa7qm".into(),
                    amount: 20000000
                }
            ]
        );

        let cmds: Vec<DbInternalCommandWithData> = internal_cmds
            .into_iter()
            .map(|cmd| {
                DbInternalCommandWithData::from_internal_cmd(
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
                DbInternalCommandWithData::Coinbase {
                    receiver: "B62qs2YyNuo1LbNo5sbhPByDDAB7NZiejFM6H1ctND5ui7wH4PWa7qm".into(),
                    amount: 720000000000,
                    state_hash: "3NLMeYAFXxsmhSFtLHFxdtjGcfHTVFmBmBF8uTJvP4Ve5yEmxYeA".into(),
                    kind: InternalCommandKind::Coinbase,
                    block_height: block.blockchain_length(),
                    date_time: block.timestamp() as i64,
                },
                DbInternalCommandWithData::FeeTransfer {
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

    #[test]
    fn from_genesis_v2() -> anyhow::Result<()> {
        let path = std::path::PathBuf::from("./data/genesis_blocks/mainnet-359605-3NK4BpDSekaqsG6tx8Nse2zJchRft2JpnbvMiog55WCr5xJZaKeP.json");
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;

        // empty internal commands
        assert_eq!(InternalCommand::from_precomputed(&block), vec![]);

        // empty db internal commands
        assert_eq!(DbInternalCommand::from_precomputed(&block), vec![]);

        // empty db internal commands with metadata
        assert_eq!(DbInternalCommandWithData::from_precomputed(&block), vec![]);

        Ok(())
    }
}
