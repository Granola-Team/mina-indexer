//! Indexer internal precomputed block representation

use crate::{
    block::{
        extract_block_height, extract_network, extract_state_hash, Block, BlockHash, VrfOutput,
    },
    canonicity::Canonicity,
    chain::Network,
    command::{signed::SignedCommand, UserCommandWithStatus, UserCommandWithStatusT},
    constants::{berkeley::*, *},
    ledger::{coinbase::Coinbase, public_key::PublicKey, username::Username, LedgerHash},
    mina_blocks::{common::from_str, v2},
    protocol::serialization_types::{
        blockchain_state::BlockchainState,
        consensus_state as mina_consensus,
        protocol_state::{ProtocolState, ProtocolStateJson},
        snark_work as mina_snark, staged_ledger_diff as mina_rs,
    },
};
use anyhow::bail;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
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
    #[serde(default = "berkeley_genesis_timestamp")]
    #[serde(deserialize_with = "from_str")]
    scheduled_time: u64,

    protocol_state: v2::protocol_state::ProtocolState,
    staged_ledger_diff: v2::staged_ledger_diff::StagedLedgerDiff,
}

fn berkeley_genesis_timestamp() -> u64 {
    BERKELEY_GENESIS_TIMESTAMP
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum PcbVersion {
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
    pub network: Network,
    pub state_hash: BlockHash,
    pub scheduled_time: u64,
    pub blockchain_length: u32,
    pub protocol_state: ProtocolState,
    pub staged_ledger_diff: mina_rs::StagedLedgerDiff,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrecomputedBlockV2 {
    pub network: Network,
    pub state_hash: BlockHash,
    pub scheduled_time: u64,
    pub blockchain_length: u32,
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
                    scheduled_time,
                    protocol_state,
                    staged_ledger_diff,
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
        let network = extract_network(path);
        let blockchain_length = extract_block_height(path).expect("length in filename");
        let state_hash = extract_state_hash(path);
        let contents = std::fs::read(path)?;
        let precomputed_block = PrecomputedBlock::from_file_contents(
            BlockFileContents {
                contents,
                blockchain_length,
                network,
                state_hash: state_hash.into(),
            },
            version,
        )?;
        Ok(precomputed_block)
    }

    pub fn commands(&self) -> Vec<UserCommandWithStatus> {
        let mut commands = self.commands_post_diff();
        commands.append(&mut self.commands_pre_diff());
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

    pub fn accounts_created(&self) -> Vec<PublicKey> {
        self.commands()
            .into_iter()
            .filter_map(|cmd| {
                let signed: SignedCommand = cmd.clone().into();
                if cmd
                    .status_data()
                    .fee_payer_account_creation_fee_paid()
                    .is_some()
                {
                    Some(signed.fee_payer_pk())
                } else if cmd
                    .status_data()
                    .receiver_account_creation_fee_paid()
                    .is_some()
                {
                    Some(signed.receiver_pk())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn consensus_state(&self) -> mina_consensus::ConsensusState {
        match self {
            Self::V1(v1) => v1.protocol_state.body.t.t.consensus_state.t.t.to_owned(),
            Self::V2(_) => todo!("consensus_state {}", self.summary()),
        }
    }

    pub fn blockchain_state(&self) -> BlockchainState {
        match self {
            Self::V1(v1) => v1.protocol_state.body.t.t.blockchain_state.t.t.to_owned(),
            Self::V2(_) => todo!("blockchain_state {}", self.summary()),
        }
    }

    pub fn block_creator(&self) -> PublicKey {
        self.consensus_state().block_creator.into()
    }

    pub fn block_stake_winner(&self) -> PublicKey {
        self.consensus_state().block_stake_winner.into()
    }

    pub fn internal_command_balances(&self) -> Vec<mina_rs::InternalCommandBalanceData> {
        self.staged_ledger_pre_diff()
            .internal_command_balances
            .iter()
            .map(|x| x.t.to_owned())
            .collect()
    }

    pub fn staged_ledger_hash(&self) -> LedgerHash {
        LedgerHash::from_hashv1(match self {
            Self::V1(v1) => v1
                .protocol_state
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
            Self::V2(_) => todo!("staged_ledger_hash {}", self.summary()),
        })
    }

    pub fn staged_ledger_diff_tuple(&self) -> mina_rs::StagedLedgerDiffTuple {
        match self {
            Self::V1(v1) => v1.staged_ledger_diff.diff.t.to_owned(),
            Self::V2(_) => todo!("staged_ledger_diff_tuple {}", self.summary()),
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
        self.consensus_state().coinbase_receiver.into()
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
                BlockHash::from_hashv1(v1.protocol_state.body.t.t.genesis_state_hash.clone())
                    .expect("genesis state hash")
            }
            Self::V2(v2) => v2.protocol_state.body.genesis_state_hash.clone(),
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
                // TODO add mainnet end slot height?
                v2.protocol_state
                    .body
                    .consensus_state
                    .global_slot_since_genesis
            }
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
                    .expect("previous state hash")
            }
            Self::V2(v2) => v2.protocol_state.previous_state_hash.to_owned(),
        }
    }

    pub fn command_hashes(&self) -> Vec<String> {
        SignedCommand::from_precomputed(self)
            .iter()
            .map(|cmd| cmd.hash_signed_command().unwrap())
            .collect()
    }

    pub fn username_updates(&self) -> HashMap<PublicKey, Username> {
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
        updates
    }

    /// Base64 encoded string
    pub fn last_vrf_output(&self) -> String {
        let last_vrf_output = VrfOutput::new(self.consensus_state().last_vrf_output.t.0.clone());
        last_vrf_output.base64_encode()
    }

    /// Blake2b hex digest of last_vrf_output
    pub fn hash_last_vrf_output(&self) -> VrfOutput {
        let last_vrf_output = VrfOutput::new(self.consensus_state().last_vrf_output.t.0.clone());
        VrfOutput::new(last_vrf_output.hex_digest())
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

impl PcbVersion {
    pub fn update(&mut self) -> anyhow::Result<()> {
        match self {
            Self::V1 => {
                *self = Self::V2;
                Ok(())
            }
            Self::V2 => bail!("No successor verion of {}", self),
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
    fn vrf_output() -> anyhow::Result<()> {
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
}
