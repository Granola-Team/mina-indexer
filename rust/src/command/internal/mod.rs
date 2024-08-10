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

        if let [coinbase_transfer, fee_transfer_via_coinbase] = &coinbase.fee_transfer()[..] {
            if let Some(idx) = account_diff_fees.windows(2).position(|pair| {
                matches!(
                    (&pair[0], &pair[1]),
                    (
                        AccountDiff::FeeTransfer(fee1),
                        AccountDiff::FeeTransfer(fee2)
                    ) if *fee1 == *coinbase_transfer && *fee2 == *fee_transfer_via_coinbase
                )
            }) {
                account_diff_fees[idx] =
                    AccountDiff::FeeTransferViaCoinbase(coinbase_transfer.clone());
                account_diff_fees[idx + 1] =
                    AccountDiff::FeeTransferViaCoinbase(fee_transfer_via_coinbase.clone());
            }
        }

        // Process the account_diff_fees into internal commands
        let mut internal_commands: Vec<InternalCommand> = account_diff_fees
            .chunks(2)
            .filter_map(|chunk| match chunk {
                [AccountDiff::FeeTransfer(a), b] => Some(Self::FeeTransfer {
                    sender: if a.update_type == UpdateType::Credit {
                        b.public_key()
                    } else {
                        a.public_key.clone()
                    },
                    receiver: if a.update_type == UpdateType::Credit {
                        a.public_key.clone()
                    } else {
                        b.public_key()
                    },
                    amount: a.amount.0 + u64::try_from(b.amount()).ok().unwrap_or(0),
                }),
                [AccountDiff::FeeTransferViaCoinbase(a), b] => Some(Self::FeeTransferViaCoinbase {
                    sender: if a.update_type == UpdateType::Credit {
                        b.public_key()
                    } else {
                        a.public_key.clone()
                    },
                    receiver: if a.update_type == UpdateType::Credit {
                        a.public_key.clone()
                    } else {
                        b.public_key()
                    },
                    amount: a.amount.0,
                }),
                [AccountDiff::Coinbase(a), _] => Some(Self::Coinbase {
                    receiver: a.public_key.clone(),
                    amount: a.amount.0,
                }),
                _ => None,
            })
            .collect();

        // Optionally add the coinbase command if it was applied
        if coinbase.is_coinbase_applied() {
            internal_commands.insert(0, coinbase.as_internal_cmd());
        }

        internal_commands
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
