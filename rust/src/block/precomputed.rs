//! Indexer internal precomputed block representation

use super::{epoch_data::EpochSeed, extract_network_height_hash, Block, BlockHash, VrfOutput};
use crate::{
    canonicity::Canonicity,
    chain::Network,
    command::{
        signed::{SignedCommand, TxnHash},
        UserCommandWithStatus, UserCommandWithStatusT,
    },
    constants::{berkeley::*, *},
    ledger::{coinbase::Coinbase, public_key::PublicKey, username::Username, LedgerHash},
    mina_blocks::{common::from_str, v2},
    protocol::serialization_types::{
        protocol_state::{ProtocolState, ProtocolStateJson},
        snark_work as mina_snark, staged_ledger_diff as mina_rs,
    },
    store::username::UsernameUpdate,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
};

pub struct BlockFileContents {
    pub(crate) network: Network,
    pub(crate) state_hash: BlockHash,
    pub(crate) blockchain_length: u32,
    pub(crate) contents: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockFileV1 {
    #[serde(default = "mainnet_genesis_timestamp")]
    #[serde(deserialize_with = "from_str")]
    scheduled_time: u64,

    protocol_state: ProtocolStateJson,
    staged_ledger_diff: mina_rs::StagedLedgerDiffJson,
}

fn mainnet_genesis_timestamp() -> u64 {
    MAINNET_GENESIS_TIMESTAMP
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockFileV2 {
    version: u32,
    data: BlockFileDataV2,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockFileDataV2 {
    #[serde(default = "berkeley_genesis_timestamp")]
    #[serde(deserialize_with = "from_str")]
    scheduled_time: u64,

    protocol_state: v2::protocol_state::ProtocolState,
    staged_ledger_diff: v2::staged_ledger_diff::StagedLedgerDiff,
}

fn berkeley_genesis_timestamp() -> u64 {
    BERKELEY_GENESIS_TIMESTAMP
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum PcbVersion {
    #[default]
    V1,
    V2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PrecomputedBlock {
    V1(Box<PrecomputedBlockV1>),
    V2(PrecomputedBlockV2),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrecomputedBlockV1 {
    // metadata
    pub network: Network,
    pub state_hash: BlockHash,
    pub blockchain_length: u32,
    // from PCB
    pub scheduled_time: u64,
    pub protocol_state: ProtocolState,
    pub staged_ledger_diff: mina_rs::StagedLedgerDiff,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrecomputedBlockV2 {
    // metadata
    pub network: Network,
    pub state_hash: BlockHash,
    pub blockchain_length: u32,
    // from PCB
    pub scheduled_time: u64,
    pub protocol_state: v2::protocol_state::ProtocolState,
    pub staged_ledger_diff: v2::staged_ledger_diff::StagedLedgerDiff,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PrecomputedBlockWithCanonicity {
    V1(Box<PrecomputedBlockWithCanonicityV1>),
    V2(PrecomputedBlockWithCanonicityV2),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrecomputedBlockWithCanonicityV1 {
    pub canonicity: Option<Canonicity>,
    pub network: Network,
    pub state_hash: BlockHash,
    pub scheduled_time: u64,
    pub blockchain_length: u32,
    pub protocol_state: ProtocolState,
    pub staged_ledger_diff: mina_rs::StagedLedgerDiff,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrecomputedBlockWithCanonicityV2 {
    pub canonicity: Option<Canonicity>,
    pub network: Network,
    pub state_hash: BlockHash,
    pub scheduled_time: u64,
    pub blockchain_length: u32,
    pub protocol_state: v2::protocol_state::ProtocolState,
    pub staged_ledger_diff: v2::staged_ledger_diff::StagedLedgerDiff,
}

impl PrecomputedBlock {
    pub fn from_file_contents(
        block_file_contents: BlockFileContents,
        version: PcbVersion,
    ) -> anyhow::Result<Self> {
        let state_hash = block_file_contents.state_hash;
        let blockchain_length = block_file_contents.blockchain_length;

        match version {
            PcbVersion::V1 => {
                let BlockFileV1 {
                    scheduled_time,
                    protocol_state,
                    staged_ledger_diff,
                } = serde_json::from_slice(&block_file_contents.contents)?;
                Ok(Self::V1(Box::new(PrecomputedBlockV1 {
                    state_hash,
                    scheduled_time,
                    blockchain_length,
                    network: block_file_contents.network,
                    protocol_state: protocol_state.to_owned().into(),
                    staged_ledger_diff: staged_ledger_diff.to_owned().into(),
                })))
            }
            PcbVersion::V2 => {
                let BlockFileV2 {
                    version: _,
                    data:
                        BlockFileDataV2 {
                            scheduled_time,
                            protocol_state,
                            staged_ledger_diff,
                        },
                } = serde_json::from_slice(&block_file_contents.contents)?;
                Ok(Self::V2(PrecomputedBlockV2 {
                    state_hash,
                    scheduled_time,
                    blockchain_length,
                    network: block_file_contents.network,
                    protocol_state: protocol_state.to_owned(),
                    staged_ledger_diff: staged_ledger_diff.to_owned(),
                }))
            }
        }
    }

    pub fn new(
        network: &str,
        blockchain_length: u32,
        state_hash: &str,
        contents: Vec<u8>,
        version: PcbVersion,
    ) -> anyhow::Result<Self> {
        let precomputed_block = PrecomputedBlock::from_file_contents(
            BlockFileContents {
                contents,
                blockchain_length,
                network: network.into(),
                state_hash: state_hash.into(),
            },
            version,
        )?;
        Ok(precomputed_block)
    }

    /// Parses the precomputed block if the path is a valid block file
    pub fn parse_file(path: &Path, version: PcbVersion) -> anyhow::Result<Self> {
        let (network, blockchain_length, state_hash) = extract_network_height_hash(path);
        let contents = std::fs::read(path)?;
        let precomputed_block = PrecomputedBlock::from_file_contents(
            BlockFileContents {
                contents,
                blockchain_length,
                network,
                state_hash,
            },
            version,
        )?;
        Ok(precomputed_block)
    }

    pub fn commands(&self) -> Vec<UserCommandWithStatus> {
        let mut commands = self.commands_pre_diff();
        commands.append(&mut self.commands_post_diff());
        commands
    }

    pub fn commands_pre_diff(&self) -> Vec<UserCommandWithStatus> {
        match self {
            Self::V1(v1) => v1
                .staged_ledger_diff
                .diff
                .clone()
                .inner()
                .0
                .inner()
                .inner()
                .commands
                .into_iter()
                .map(UserCommandWithStatus)
                .collect(),
            Self::V2(_) => todo!("commands_pre_diff {}", self.summary()),
        }
    }

    pub fn commands_post_diff(&self) -> Vec<UserCommandWithStatus> {
        match self {
            Self::V1(v1) => v1
                .staged_ledger_diff
                .diff
                .clone()
                .inner()
                .1
                .map_or(vec![], |diff| {
                    diff.inner()
                        .inner()
                        .commands
                        .into_iter()
                        .map(UserCommandWithStatus)
                        .collect()
                }),
            Self::V2(_) => todo!("commands_post_diff {}", self.summary()),
        }
    }

    pub fn tx_fees(&self) -> u64 {
        self.commands()
            .into_iter()
            .map(|cmd| {
                let signed: SignedCommand = cmd.clone().into();
                signed.fee()
            })
            .sum()
    }

    pub fn snark_fees(&self) -> u64 {
        self.completed_works()
            .into_iter()
            .map(|work| work.fee.t.t)
            .sum()
    }

    /// Returns the pair of
    /// - new pk balances (after applying coinbase, before fee transfers)
    /// - new coinbase receiver option
    pub fn accounts_created(&self) -> (BTreeMap<PublicKey, u64>, Option<PublicKey>) {
        let mut new_coinbase_receiver = None;
        let mut account_balances = BTreeMap::new();

        // maybe coinbase receiver
        if let Some(bal) = self.coinbase_receiver_balance() {
            if [
                MAINNET_COINBASE_REWARD - MAINNET_ACCOUNT_CREATION_FEE.0,
                // supercharged
                2 * MAINNET_COINBASE_REWARD - MAINNET_ACCOUNT_CREATION_FEE.0,
            ]
            .contains(&bal)
            {
                account_balances.insert(self.coinbase_receiver(), bal);
                new_coinbase_receiver = Some(self.coinbase_receiver());
            }
        }

        // from user commands
        self.commands().iter().for_each(|cmd| {
            let status = cmd.status_data();
            let signed: SignedCommand = cmd.clone().into();

            if status.fee_payer_account_creation_fee_paid().is_some() {
                account_balances.insert(
                    signed.fee_payer_pk(),
                    status.fee_payer_balance().unwrap_or_default(),
                );
            } else if status.receiver_account_creation_fee_paid().is_some() {
                account_balances.insert(
                    signed.receiver_pk(),
                    status.receiver_balance().unwrap_or_default(),
                );
            }
        });
        (account_balances, new_coinbase_receiver)
    }

    //////////////////////
    // Blockchain state //
    //////////////////////

    pub fn snarked_ledger_hash(&self) -> Option<LedgerHash> {
        match self {
            Self::V1(v1) => Some(LedgerHash::from_hashv1(
                v1.protocol_state
                    .body
                    .t
                    .t
                    .blockchain_state
                    .t
                    .t
                    .snarked_ledger_hash
                    .to_owned(),
            )),
            Self::V2(_v2) => None,
        }
    }

    pub fn staged_ledger_hash(&self) -> LedgerHash {
        match self {
            Self::V1(v1) => LedgerHash::from_hashv1(
                v1.protocol_state
                    .body
                    .t
                    .t
                    .blockchain_state
                    .t
                    .t
                    .staged_ledger_hash
                    .t
                    .t
                    .non_snark
                    .t
                    .ledger_hash
                    .to_owned(),
            ),
            Self::V2(v2) => v2
                .protocol_state
                .body
                .blockchain_state
                .staged_ledger_hash
                .non_snark
                .ledger_hash
                .to_owned(),
        }
    }

    /////////////////////
    // Consensus state //
    /////////////////////

    pub fn total_currency(&self) -> u64 {
        match self {
            Self::V1(v1) => {
                v1.protocol_state
                    .body
                    .t
                    .t
                    .consensus_state
                    .t
                    .t
                    .total_currency
                    .t
                    .t
            }
            Self::V2(v2) => v2.protocol_state.body.consensus_state.total_currency,
        }
    }

    pub fn block_creator(&self) -> PublicKey {
        match self {
            Self::V1(v1) => v1
                .protocol_state
                .body
                .t
                .t
                .consensus_state
                .t
                .t
                .block_creator
                .to_owned()
                .into(),
            Self::V2(v2) => v2
                .protocol_state
                .body
                .consensus_state
                .block_creator
                .to_owned(),
        }
    }

    pub fn block_stake_winner(&self) -> PublicKey {
        match self {
            Self::V1(v1) => v1
                .protocol_state
                .body
                .t
                .t
                .consensus_state
                .t
                .t
                .block_stake_winner
                .to_owned()
                .into(),
            Self::V2(v2) => v2
                .protocol_state
                .body
                .consensus_state
                .block_stake_winner
                .to_owned(),
        }
    }

    pub fn has_ancestor_in_same_checkpoint_window(&self) -> bool {
        match self {
            Self::V1(v1) => {
                v1.protocol_state
                    .body
                    .t
                    .t
                    .consensus_state
                    .t
                    .t
                    .has_ancestor_in_same_checkpoint_window
            }
            Self::V2(v2) => {
                v2.protocol_state
                    .body
                    .consensus_state
                    .has_ancestor_in_same_checkpoint_window
            }
        }
    }

    pub fn internal_command_balances(&self) -> Vec<mina_rs::InternalCommandBalanceData> {
        self.staged_ledger_pre_diff()
            .internal_command_balances
            .iter()
            .map(|x| x.t.to_owned())
            .collect()
    }

    pub fn staged_ledger_diff_tuple(&self) -> mina_rs::StagedLedgerDiffTuple {
        match self {
            Self::V1(v1) => v1.staged_ledger_diff.diff.t.to_owned(),
            Self::V2(_v2) => todo!("V2 staged_ledger_diff_tuple {}", self.summary()),
        }
    }

    pub fn staged_ledger_pre_diff(&self) -> mina_rs::StagedLedgerPreDiff {
        self.staged_ledger_diff_tuple().0.inner().inner()
    }

    pub fn staged_ledger_post_diff(&self) -> Option<mina_rs::StagedLedgerPreDiff> {
        self.staged_ledger_diff_tuple().1.map(|x| x.inner().inner())
    }

    pub fn completed_works(&self) -> Vec<mina_snark::TransactionSnarkWork> {
        let mut completed_works = self.completed_works_post_diff().unwrap_or_default();
        completed_works.append(&mut self.completed_works_pre_diff());
        completed_works
    }

    pub fn completed_works_pre_diff(&self) -> Vec<mina_snark::TransactionSnarkWork> {
        self.staged_ledger_pre_diff()
            .completed_works
            .iter()
            .map(|x| x.t.clone())
            .collect()
    }

    pub fn completed_works_post_diff(&self) -> Option<Vec<mina_snark::TransactionSnarkWork>> {
        self.staged_ledger_post_diff()
            .map(|diff| diff.completed_works.iter().map(|x| x.t.clone()).collect())
    }

    pub fn coinbase_receiver_balance(&self) -> Option<u64> {
        for internal_balance in self.internal_command_balances() {
            if let mina_rs::InternalCommandBalanceData::CoinBase(x) = internal_balance {
                return Some(x.inner().coinbase_receiver_balance.inner().inner().inner());
            }
        }

        None
    }

    pub fn fee_transfer_balances(&self) -> Vec<(u64, Option<u64>)> {
        let mut res = vec![];
        for internal_balance in self.internal_command_balances() {
            if let mina_rs::InternalCommandBalanceData::FeeTransfer(x) = internal_balance {
                res.push((
                    x.t.receiver1_balance.t.t.t,
                    x.t.receiver2_balance.map(|balance| balance.t.t.t),
                ));
            }
        }
        res
    }

    pub fn coinbase_receiver(&self) -> PublicKey {
        match self {
            Self::V1(v1) => v1
                .protocol_state
                .body
                .t
                .t
                .consensus_state
                .t
                .t
                .coinbase_receiver
                .to_owned()
                .into(),
            Self::V2(v2) => v2
                .protocol_state
                .body
                .consensus_state
                .coinbase_receiver
                .to_owned(),
        }
    }

    pub fn supercharge_coinbase(&self) -> bool {
        match self {
            Self::V1(v1) => {
                v1.protocol_state
                    .body
                    .t
                    .t
                    .consensus_state
                    .t
                    .t
                    .supercharge_coinbase
            }
            Self::V2(v2) => v2.protocol_state.body.consensus_state.supercharge_coinbase,
        }
    }

    pub fn consensus_public_keys(&self) -> HashSet<PublicKey> {
        HashSet::from([
            self.block_creator(),
            self.coinbase_receiver(),
            self.block_stake_winner(),
        ])
    }

    /// All applied & failed command public keys
    pub fn all_command_public_keys(&self) -> Vec<PublicKey> {
        let mut pk_set: HashSet<PublicKey> = self.consensus_public_keys();

        // add keys from all commands
        let commands = self.commands();
        commands.iter().for_each(|command| {
            let signed_command = match command.clone().data() {
                mina_rs::UserCommand::SignedCommand(signed_command) => {
                    SignedCommand(signed_command)
                }
            };
            add_keys(&mut pk_set, signed_command.all_command_public_keys());
        });

        let mut pks: Vec<PublicKey> = pk_set.into_iter().collect();
        pks.sort();
        pks
    }

    /// Prover public keys for completed SNARK work
    pub fn prover_keys(&self) -> Vec<PublicKey> {
        let mut pk_set: HashSet<PublicKey> = self.consensus_public_keys();

        // add prover keys from completed SNARK work
        let completed_works = self.completed_works();
        completed_works.iter().for_each(|work| {
            pk_set.insert(work.prover.clone().into());
        });

        let mut pks: Vec<PublicKey> = pk_set.into_iter().collect();
        pks.sort();
        pks
    }

    /// Vec of public keys which send or receive funds in applied commands and
    /// coinbase
    pub fn active_public_keys(&self) -> Vec<PublicKey> {
        // block creator and block stake winner
        let mut public_keys: HashSet<PublicKey> =
            HashSet::from([self.block_creator(), self.block_stake_winner()]);

        // coinbase receiver if coinbase is applied
        if Coinbase::from_precomputed(self).is_coinbase_applied() {
            public_keys.insert(self.coinbase_receiver());
        }

        // applied commands
        self.commands()
            .iter()
            .filter(|cmd| cmd.is_applied())
            .for_each(|command| {
                let signed_command = match command.clone().data() {
                    mina_rs::UserCommand::SignedCommand(signed_command) => {
                        SignedCommand(signed_command)
                    }
                };
                add_keys(&mut public_keys, signed_command.all_command_public_keys());
            });

        let mut pks: Vec<PublicKey> = public_keys.into_iter().collect();
        pks.sort();
        pks
    }

    pub fn all_public_keys(&self) -> Vec<PublicKey> {
        let mut public_keys: HashSet<PublicKey> =
            self.all_command_public_keys().into_iter().collect();
        add_keys(&mut public_keys, self.prover_keys());

        let mut public_keys: Vec<PublicKey> = public_keys.into_iter().collect();
        public_keys.sort();
        public_keys
    }

    pub fn genesis_state_hash(&self) -> BlockHash {
        match self {
            Self::V1(v1) => {
                BlockHash::from_hashv1(v1.protocol_state.body.t.t.genesis_state_hash.to_owned())
            }
            Self::V2(v2) => v2.protocol_state.body.genesis_state_hash.to_owned(),
        }
    }

    pub fn global_slot_since_genesis(&self) -> u32 {
        match self {
            Self::V1(v1) => {
                v1.protocol_state
                    .body
                    .t
                    .t
                    .consensus_state
                    .t
                    .t
                    .global_slot_since_genesis
                    .t
                    .t
            }
            Self::V2(v2) => {
                v2.protocol_state
                    .body
                    .consensus_state
                    .global_slot_since_genesis
            }
        }
    }

    pub fn min_window_density(&self) -> u32 {
        match self {
            Self::V1(v1) => {
                v1.protocol_state
                    .body
                    .t
                    .t
                    .consensus_state
                    .t
                    .t
                    .min_window_density
                    .t
                    .t
            }
            Self::V2(v2) => v2.protocol_state.body.consensus_state.min_window_density,
        }
    }

    // next epoch data

    pub fn next_epoch_seed(&self) -> String {
        match self {
            Self::V1(v1) => EpochSeed::from_hashv1(
                v1.protocol_state
                    .body
                    .t
                    .t
                    .consensus_state
                    .t
                    .t
                    .next_epoch_data
                    .t
                    .t
                    .seed
                    .to_owned(),
            ),
            Self::V2(v2) => v2
                .protocol_state
                .body
                .consensus_state
                .next_epoch_data
                .seed
                .to_owned(),
        }
    }

    pub fn next_epoch_ledger_hash(&self) -> LedgerHash {
        match self {
            Self::V1(v1) => LedgerHash::from_hashv1(
                v1.protocol_state
                    .body
                    .t
                    .t
                    .consensus_state
                    .t
                    .t
                    .next_epoch_data
                    .t
                    .t
                    .ledger
                    .t
                    .t
                    .hash
                    .to_owned(),
            ),
            Self::V2(v2) => v2
                .protocol_state
                .body
                .consensus_state
                .next_epoch_data
                .ledger
                .hash
                .to_owned(),
        }
    }

    pub fn next_epoch_length(&self) -> u32 {
        match self {
            Self::V1(v1) => {
                v1.protocol_state
                    .body
                    .t
                    .t
                    .consensus_state
                    .t
                    .t
                    .next_epoch_data
                    .t
                    .t
                    .epoch_length
                    .t
                    .t
            }
            Self::V2(v2) => {
                v2.protocol_state
                    .body
                    .consensus_state
                    .next_epoch_data
                    .epoch_length
            }
        }
    }

    pub fn next_epoch_total_currency(&self) -> u64 {
        match self {
            Self::V1(v1) => {
                v1.protocol_state
                    .body
                    .t
                    .t
                    .consensus_state
                    .t
                    .t
                    .next_epoch_data
                    .t
                    .t
                    .ledger
                    .t
                    .t
                    .total_currency
                    .t
                    .t
            }
            Self::V2(v2) => {
                v2.protocol_state
                    .body
                    .consensus_state
                    .next_epoch_data
                    .ledger
                    .total_currency
            }
        }
    }

    pub fn next_epoch_start_checkpoint(&self) -> BlockHash {
        match self {
            Self::V1(v1) => BlockHash::from_hashv1(
                v1.protocol_state
                    .body
                    .t
                    .t
                    .consensus_state
                    .t
                    .t
                    .next_epoch_data
                    .t
                    .t
                    .start_checkpoint
                    .to_owned(),
            ),
            Self::V2(v2) => v2
                .protocol_state
                .body
                .consensus_state
                .next_epoch_data
                .start_checkpoint
                .to_owned(),
        }
    }

    pub fn next_epoch_lock_checkpoint(&self) -> BlockHash {
        match self {
            Self::V1(v1) => BlockHash::from_hashv1(
                v1.protocol_state
                    .body
                    .t
                    .t
                    .consensus_state
                    .t
                    .t
                    .next_epoch_data
                    .t
                    .t
                    .lock_checkpoint
                    .to_owned(),
            ),
            Self::V2(v2) => v2
                .protocol_state
                .body
                .consensus_state
                .next_epoch_data
                .lock_checkpoint
                .to_owned(),
        }
    }

    // staking epoch data

    pub fn staking_epoch_length(&self) -> u32 {
        match self {
            Self::V1(v1) => {
                v1.protocol_state
                    .body
                    .t
                    .t
                    .consensus_state
                    .t
                    .t
                    .next_epoch_data
                    .t
                    .t
                    .epoch_length
                    .t
                    .t
            }
            Self::V2(v2) => {
                v2.protocol_state
                    .body
                    .consensus_state
                    .next_epoch_data
                    .epoch_length
            }
        }
    }

    pub fn staking_epoch_ledger_hash(&self) -> LedgerHash {
        match self {
            Self::V1(v1) => LedgerHash::from_hashv1(
                v1.protocol_state
                    .body
                    .t
                    .t
                    .consensus_state
                    .t
                    .t
                    .staking_epoch_data
                    .t
                    .t
                    .ledger
                    .t
                    .t
                    .hash
                    .to_owned(),
            ),
            Self::V2(v2) => v2
                .protocol_state
                .body
                .consensus_state
                .staking_epoch_data
                .ledger
                .hash
                .to_owned(),
        }
    }

    pub fn staking_epoch_seed(&self) -> String {
        match self {
            Self::V1(v1) => EpochSeed::from_hashv1(
                v1.protocol_state
                    .body
                    .t
                    .t
                    .consensus_state
                    .t
                    .t
                    .staking_epoch_data
                    .t
                    .t
                    .seed
                    .to_owned(),
            ),
            Self::V2(v2) => v2
                .protocol_state
                .body
                .consensus_state
                .staking_epoch_data
                .seed
                .to_owned(),
        }
    }

    pub fn staking_epoch_total_currency(&self) -> u64 {
        match self {
            Self::V1(v1) => {
                v1.protocol_state
                    .body
                    .t
                    .t
                    .consensus_state
                    .t
                    .t
                    .staking_epoch_data
                    .t
                    .t
                    .ledger
                    .t
                    .t
                    .total_currency
                    .t
                    .t
            }
            Self::V2(v2) => {
                v2.protocol_state
                    .body
                    .consensus_state
                    .staking_epoch_data
                    .ledger
                    .total_currency
            }
        }
    }

    pub fn staking_epoch_start_checkpoint(&self) -> BlockHash {
        match self {
            Self::V1(v1) => BlockHash::from_hashv1(
                v1.protocol_state
                    .body
                    .t
                    .t
                    .consensus_state
                    .t
                    .t
                    .staking_epoch_data
                    .t
                    .t
                    .start_checkpoint
                    .to_owned(),
            ),
            Self::V2(v2) => v2
                .protocol_state
                .body
                .consensus_state
                .staking_epoch_data
                .start_checkpoint
                .to_owned(),
        }
    }

    pub fn staking_epoch_lock_checkpoint(&self) -> BlockHash {
        match self {
            Self::V1(v1) => BlockHash::from_hashv1(
                v1.protocol_state
                    .body
                    .t
                    .t
                    .consensus_state
                    .t
                    .t
                    .staking_epoch_data
                    .t
                    .t
                    .lock_checkpoint
                    .to_owned(),
            ),
            Self::V2(v2) => v2
                .protocol_state
                .body
                .consensus_state
                .staking_epoch_data
                .lock_checkpoint
                .to_owned(),
        }
    }

    pub fn epoch_count(&self) -> u32 {
        match self {
            Self::V1(v1) => {
                v1.protocol_state
                    .body
                    .t
                    .t
                    .consensus_state
                    .t
                    .t
                    .epoch_count
                    .t
                    .t
            }
            Self::V2(v2) => v2.protocol_state.body.consensus_state.epoch_count,
        }
    }

    pub fn timestamp(&self) -> u64 {
        match self {
            Self::V1(v1) => {
                v1.protocol_state
                    .body
                    .t
                    .t
                    .blockchain_state
                    .t
                    .t
                    .timestamp
                    .t
                    .t
            }
            Self::V2(v2) => v2.protocol_state.body.blockchain_state.timestamp,
        }
    }

    pub fn scheduled_time(&self) -> String {
        match self {
            Self::V1(v1) => v1.scheduled_time.to_string(),
            Self::V2(v2) => v2.scheduled_time.to_string(),
        }
    }

    pub fn previous_state_hash(&self) -> BlockHash {
        match self {
            Self::V1(v1) => {
                BlockHash::from_hashv1(v1.protocol_state.previous_state_hash.to_owned())
            }
            Self::V2(v2) => v2.protocol_state.previous_state_hash.to_owned(),
        }
    }

    pub fn command_hashes(&self) -> Vec<TxnHash> {
        SignedCommand::from_precomputed(self)
            .iter()
            .filter_map(|cmd| cmd.signed_command.hash_signed_command().ok())
            .collect()
    }

    pub fn username_updates(&self) -> UsernameUpdate {
        let mut updates = HashMap::new();
        self.commands().iter().for_each(|cmd| {
            // check for the special name service txns
            if cmd.is_applied() {
                let sender = cmd.sender();
                let receiver = cmd.receiver();
                let memo = cmd.memo();
                if memo.starts_with(NAME_SERVICE_MEMO_PREFIX)
                    && (receiver.0 == MINA_EXPLORER_NAME_SERVICE_ADDRESS
                        || receiver.0 == MINA_SEARCH_NAME_SERVICE_ADDRESS)
                {
                    updates.insert(
                        sender,
                        Username(memo[NAME_SERVICE_MEMO_PREFIX.len()..].to_string()),
                    );
                }
            }
        });
        UsernameUpdate(updates)
    }

    /// Base64 encoded string
    pub fn last_vrf_output(&self) -> String {
        match self {
            Self::V1(v1) => {
                let last_vrf_output = VrfOutput::new(
                    v1.protocol_state
                        .body
                        .t
                        .t
                        .consensus_state
                        .t
                        .t
                        .last_vrf_output
                        .t
                        .0
                        .to_owned(),
                );
                last_vrf_output.base64_encode()
            }
            Self::V2(v2) => v2
                .protocol_state
                .body
                .consensus_state
                .last_vrf_output
                .to_owned(),
        }
    }

    /// Blake2b hex digest of last_vrf_output
    pub fn hash_last_vrf_output(&self) -> VrfOutput {
        match self {
            Self::V1(v1) => {
                let last_vrf_output = VrfOutput::new(
                    v1.protocol_state
                        .body
                        .t
                        .t
                        .consensus_state
                        .t
                        .t
                        .last_vrf_output
                        .t
                        .0
                        .to_owned(),
                );
                VrfOutput::new(last_vrf_output.hex_digest())
            }
            Self::V2(v2) => {
                VrfOutput::base64_decode(&v2.protocol_state.body.consensus_state.last_vrf_output)
                    .expect("V2 last VRF output decodes")
            }
        }
    }

    pub fn with_canonicity(&self, canonicity: Canonicity) -> PrecomputedBlockWithCanonicity {
        match self {
            Self::V1(v1) => {
                PrecomputedBlockWithCanonicity::V1(Box::new(PrecomputedBlockWithCanonicityV1 {
                    canonicity: Some(canonicity),
                    network: v1.network.to_owned(),
                    state_hash: v1.state_hash.to_owned(),
                    blockchain_length: v1.blockchain_length,
                    scheduled_time: v1.scheduled_time,
                    protocol_state: v1.protocol_state.to_owned(),
                    staged_ledger_diff: v1.staged_ledger_diff.to_owned(),
                }))
            }
            Self::V2(pcb_v2) => {
                PrecomputedBlockWithCanonicity::V2(PrecomputedBlockWithCanonicityV2 {
                    canonicity: Some(canonicity),
                    network: pcb_v2.network.to_owned(),
                    state_hash: pcb_v2.state_hash.to_owned(),
                    blockchain_length: pcb_v2.blockchain_length,
                    scheduled_time: pcb_v2.scheduled_time,
                    protocol_state: pcb_v2.protocol_state.to_owned(),
                    staged_ledger_diff: pcb_v2.staged_ledger_diff.to_owned(),
                })
            }
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "{} (length {}): {}",
            self.network(),
            self.blockchain_length(),
            self.state_hash()
        )
    }

    pub fn state_hash(&self) -> BlockHash {
        match self {
            PrecomputedBlock::V1(v1) => v1.state_hash.to_owned(),
            PrecomputedBlock::V2(v2) => v2.state_hash.to_owned(),
        }
    }

    pub fn blockchain_length(&self) -> u32 {
        match self {
            PrecomputedBlock::V1(v1) => v1.blockchain_length,
            PrecomputedBlock::V2(v2) => v2.blockchain_length,
        }
    }

    pub fn network(&self) -> Network {
        match self {
            PrecomputedBlock::V1(v1) => v1.network.to_owned(),
            PrecomputedBlock::V2(v2) => v2.network.to_owned(),
        }
    }

    pub fn version(&self) -> PcbVersion {
        match self {
            Self::V1(_) => PcbVersion::V1,
            Self::V2(_) => PcbVersion::V2,
        }
    }
}

impl std::cmp::PartialOrd for PrecomputedBlock {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for PrecomputedBlock {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_block: Block = self.into();
        let other_block: Block = other.into();
        self_block.cmp(&other_block)
    }
}

impl std::cmp::Eq for PrecomputedBlock {}

impl std::fmt::Display for PcbVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V1 => write!(f, "v1"),
            Self::V2 => write!(f, "v2"),
        }
    }
}

fn add_keys(pks: &mut HashSet<PublicKey>, new_pks: Vec<PublicKey>) {
    for pk in new_pks {
        pks.insert(pk);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;
    use std::path::PathBuf;

    #[test]
    fn vrf_output_v1() -> anyhow::Result<()> {
        let path: PathBuf = "./tests/data/sequential_blocks/mainnet-105489-3NLFXtdzaFW2WX6KgrxMjL4enE4pCa9hAsVUPm47PT6337SXgBGh.json".into();
        let block = PrecomputedBlock::parse_file(&path, PcbVersion::V1)?;
        assert_eq!(
            block.last_vrf_output(),
            "bgHnww8tqHDhk3rBpW9tse_L_WPup7yKDKigNvoeBwA=".to_string()
        );
        assert_eq!(
            block.hash_last_vrf_output(),
            VrfOutput::new(
                hex!("7b0bc721df63c1eabf5b85c0e05e952c6b06c1aa101db1ed3acea4faaf8420c4").to_vec()
            )
        );
        Ok(())
    }

    #[test]
    fn vrf_output_v2() -> anyhow::Result<()> {
        let path = PathBuf::from("./tests/data/berkeley/sequential_blocks/berkeley-2-3NLBi19dn8P4Fm5UZgd2gdmi1WbuxyM1uuk2ci1zEwP4iEijHEwJ.json");
        let pcb = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;
        assert_eq!(
            pcb.last_vrf_output(),
            "rWxD4L_t-VXaoDDVJipD5OR9OU6X4T6WwEWCxvoEAAA=".to_string()
        );
        Ok(())
    }
}
