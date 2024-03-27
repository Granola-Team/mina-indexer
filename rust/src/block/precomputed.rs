use crate::{
    block::{
        extract_block_height, extract_network, extract_state_hash, Block, BlockHash, VrfOutput,
    },
    canonicity::Canonicity,
    command::{signed::SignedCommand, UserCommandWithStatus},
    constants::MAINNET_GENESIS_TIMESTAMP,
    ledger::{coinbase::Coinbase, public_key::PublicKey, LedgerHash},
    protocol::serialization_types::{
        consensus_state as mina_consensus,
        protocol_state::{ProtocolState, ProtocolStateJson},
        snark_work as mina_snark, staged_ledger_diff as mina_rs,
    },
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path};

pub struct BlockFileContents {
    pub(crate) network: String,
    pub(crate) state_hash: BlockHash,
    pub(crate) blockchain_length: u32,
    pub(crate) contents: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockFile {
    #[serde(default = "genesis_timestamp")]
    scheduled_time: String,
    protocol_state: ProtocolStateJson,
    staged_ledger_diff: mina_rs::StagedLedgerDiffJson,
}

fn genesis_timestamp() -> String {
    MAINNET_GENESIS_TIMESTAMP.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrecomputedBlock {
    pub network: String,
    pub state_hash: String,
    pub scheduled_time: String,
    pub blockchain_length: u32,
    pub protocol_state: ProtocolState,
    pub staged_ledger_diff: mina_rs::StagedLedgerDiff,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrecomputedBlockWithCanonicity {
    pub canonicity: Option<Canonicity>,
    pub network: String,
    pub state_hash: String,
    pub scheduled_time: String,
    pub blockchain_length: u32,
    pub protocol_state: ProtocolState,
    pub staged_ledger_diff: mina_rs::StagedLedgerDiff,
}

impl PrecomputedBlock {
    pub fn from_file_contents(block_file_contents: BlockFileContents) -> serde_json::Result<Self> {
        let state_hash = block_file_contents.state_hash.0;
        let BlockFile {
            scheduled_time,
            protocol_state,
            staged_ledger_diff,
        } = serde_json::from_slice(&block_file_contents.contents)?;
        let blockchain_length = block_file_contents.blockchain_length;
        Ok(Self {
            state_hash,
            scheduled_time,
            blockchain_length,
            network: block_file_contents.network,
            protocol_state: protocol_state.into(),
            staged_ledger_diff: staged_ledger_diff.into(),
        })
    }

    /// Parses the precomputed block if the path is a valid block file
    pub fn parse_file(path: &Path) -> anyhow::Result<Self> {
        let network = extract_network(path);
        let blockchain_length = extract_block_height(path).expect("length in filename");
        let state_hash = extract_state_hash(path);
        let contents = std::fs::read(path)?;
        let precomputed_block = PrecomputedBlock::from_file_contents(BlockFileContents {
            network,
            contents,
            blockchain_length,
            state_hash: state_hash.into(),
        })?;
        Ok(precomputed_block)
    }

    pub fn commands(&self) -> Vec<UserCommandWithStatus> {
        let mut commands = self.commands_post_diff();
        commands.append(&mut self.commands_pre_diff());
        commands
    }

    pub fn commands_pre_diff(&self) -> Vec<UserCommandWithStatus> {
        self.staged_ledger_diff
            .diff
            .clone()
            .inner()
            .0
            .inner()
            .inner()
            .commands
            .into_iter()
            .map(UserCommandWithStatus)
            .collect()
    }

    pub fn commands_post_diff(&self) -> Vec<UserCommandWithStatus> {
        self.staged_ledger_diff
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
            })
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
        self.protocol_state
            .body
            .clone()
            .inner()
            .inner()
            .consensus_state
            .inner()
            .inner()
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
            .map(|x| x.t.clone())
            .collect()
    }

    pub fn staged_ledger_hash(&self) -> LedgerHash {
        LedgerHash::from_hashv1(
            self.protocol_state
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
                .clone(),
        )
    }

    pub fn staged_ledger_diff_tuple(&self) -> mina_rs::StagedLedgerDiffTuple {
        self.staged_ledger_diff.diff.clone().inner()
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

    fn consensus_public_keys(&self) -> HashSet<PublicKey> {
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

    pub fn global_slot_since_genesis(&self) -> u32 {
        self.protocol_state
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

    pub fn timestamp(&self) -> u64 {
        self.protocol_state
            .body
            .clone()
            .inner()
            .inner()
            .blockchain_state
            .inner()
            .inner()
            .timestamp
            .inner()
            .inner()
    }

    pub fn previous_state_hash(&self) -> BlockHash {
        BlockHash::from_hashv1(self.protocol_state.previous_state_hash.clone())
    }

    pub fn command_hashes(&self) -> Vec<String> {
        SignedCommand::from_precomputed(self)
            .iter()
            .map(|cmd| cmd.hash_signed_command().unwrap())
            .collect()
    }

    /// Base64 encoded string
    pub fn last_vrf_output(&self) -> String {
        let last_vrf_output = VrfOutput::new(
            self.protocol_state
                .clone()
                .body
                .inner()
                .inner()
                .consensus_state
                .inner()
                .inner()
                .last_vrf_output
                .inner()
                .0,
        );
        last_vrf_output.base64_encode()
    }

    /// Blake2b hex digest of last_vrf_output
    pub fn hash_last_vrf_output(&self) -> VrfOutput {
        let last_vrf_output = VrfOutput::new(
            self.protocol_state
                .clone()
                .body
                .t
                .t
                .consensus_state
                .t
                .t
                .last_vrf_output
                .t
                .0,
        );
        VrfOutput::new(last_vrf_output.hex_digest())
    }

    pub fn with_canonicity(&self, canonicity: Canonicity) -> PrecomputedBlockWithCanonicity {
        PrecomputedBlockWithCanonicity {
            canonicity: Some(canonicity),
            network: self.network.clone(),
            state_hash: self.state_hash.clone(),
            blockchain_length: self.blockchain_length,
            scheduled_time: self.scheduled_time.clone(),
            protocol_state: self.protocol_state.clone(),
            staged_ledger_diff: self.staged_ledger_diff.clone(),
        }
    }

    pub fn summary(&self) -> String {
        format!("(length {}): {}", self.blockchain_length, self.state_hash)
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

fn add_keys(pks: &mut HashSet<PublicKey>, new_pks: Vec<PublicKey>) {
    for pk in new_pks {
        pks.insert(pk);
    }
}

#[cfg(test)]
mod tests {
    use super::{PrecomputedBlock, VrfOutput};
    use hex_literal::hex;
    use std::path::PathBuf;

    #[test]
    fn vrf_output() -> anyhow::Result<()> {
        let path: PathBuf = "./tests/data/sequential_blocks/mainnet-105489-3NLFXtdzaFW2WX6KgrxMjL4enE4pCa9hAsVUPm47PT6337SXgBGh.json".into();
        let block = PrecomputedBlock::parse_file(&path)?;
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
