use crate::{
    block::{get_blockchain_length, get_state_hash, is_valid_block_file, BlockHash},
    canonicity::Canonicity,
    command::{signed::SignedCommand, UserCommandWithStatus},
    constants::MAINNET_GENESIS_TIMESTAMP,
    ledger::{coinbase::Coinbase, public_key::PublicKey},
};
use anyhow::anyhow;
use mina_serialization_types::{
    consensus_state as mina_consensus,
    json::DeltaTransitionChainProofJson,
    protocol_state::{ProtocolState, ProtocolStateJson},
    protocol_state_proof::ProtocolStateProofBase64Json,
    snark_work as mina_snark, staged_ledger_diff as mina_rs,
    v1::{DeltaTransitionChainProof, ProtocolStateProofV1},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path};

pub struct BlockFileContents {
    pub(crate) state_hash: String,
    pub(crate) blockchain_length: Option<u32>,
    pub(crate) contents: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockFile {
    #[serde(default = "genesis_timestamp")]
    scheduled_time: String,
    protocol_state: ProtocolStateJson,
    protocol_state_proof: ProtocolStateProofBase64Json,
    staged_ledger_diff: mina_rs::StagedLedgerDiffJson,
    delta_transition_chain_proof: DeltaTransitionChainProofJson,
}

fn genesis_timestamp() -> String {
    MAINNET_GENESIS_TIMESTAMP.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrecomputedBlock {
    pub state_hash: String,
    pub scheduled_time: String,
    pub blockchain_length: u32,
    pub protocol_state: ProtocolState,
    pub protocol_state_proof: ProtocolStateProofV1,
    pub staged_ledger_diff: mina_rs::StagedLedgerDiff,
    pub delta_transition_chain_proof: DeltaTransitionChainProof,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrecomputedBlockWithCanonicity {
    pub canonicity: Option<Canonicity>,
    pub state_hash: String,
    pub scheduled_time: String,
    pub blockchain_length: u32,
    pub protocol_state: ProtocolState,
    pub protocol_state_proof: ProtocolStateProofV1,
    pub staged_ledger_diff: mina_rs::StagedLedgerDiff,
    pub delta_transition_chain_proof: DeltaTransitionChainProof,
}

impl PrecomputedBlock {
    pub fn from_file_contents(log_contents: BlockFileContents) -> serde_json::Result<Self> {
        let state_hash = log_contents.state_hash;
        let contents = String::from_utf8_lossy(&log_contents.contents);
        let BlockFile {
            scheduled_time,
            protocol_state,
            protocol_state_proof,
            staged_ledger_diff,
            delta_transition_chain_proof,
        } = serde_json::from_str(&contents)?;
        let blockchain_length = if let Some(blockchain_length) = log_contents.blockchain_length {
            blockchain_length
        } else {
            protocol_state.body.consensus_state.blockchain_length.0
        };
        Ok(Self {
            state_hash,
            scheduled_time,
            blockchain_length,
            protocol_state: protocol_state.into(),
            protocol_state_proof: protocol_state_proof.into(),
            staged_ledger_diff: staged_ledger_diff.into(),
            delta_transition_chain_proof: delta_transition_chain_proof.into(),
        })
    }

    /// Parses the precomputed block if the path is a valid block file
    pub fn parse_file(path: &Path) -> anyhow::Result<Self> {
        if is_valid_block_file(path) {
            let file_name = path.file_name().expect("filename already checked");
            let blockchain_length = get_blockchain_length(file_name);
            let state_hash = get_state_hash(file_name).expect("state hash already checked");
            let log_file_contents = std::fs::read(path)?;
            let precomputed_block = PrecomputedBlock::from_file_contents(BlockFileContents {
                state_hash,
                blockchain_length,
                contents: log_file_contents,
            })?;
            Ok(precomputed_block)
        } else {
            Err(anyhow!(
                "Invalid precomputed block file name: {}",
                path.display()
            ))
        }
    }

    pub fn commands(&self) -> Vec<UserCommandWithStatus> {
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
        let mut completed_works: Vec<mina_snark::TransactionSnarkWork> = self
            .staged_ledger_pre_diff()
            .completed_works
            .iter()
            .map(|x| x.t.clone())
            .collect();
        let mut other = self
            .staged_ledger_post_diff()
            .map(|diff| diff.completed_works.iter().map(|x| x.t.clone()).collect())
            .unwrap_or(vec![]);
        completed_works.append(&mut other);
        completed_works
    }

    pub fn coinbase_receiver_balance(&self) -> Option<u64> {
        for internal_balance in self.internal_command_balances() {
            if let mina_rs::InternalCommandBalanceData::CoinBase(x) = internal_balance {
                return Some(x.inner().coinbase_receiver_balance.inner().inner().inner());
            }
        }

        None
    }

    pub fn fee_transfer_receiver1_balance(&self) -> Option<u64> {
        for internal_balance in self.internal_command_balances() {
            if let mina_rs::InternalCommandBalanceData::FeeTransfer(x) = internal_balance {
                return Some(x.inner().receiver1_balance.inner().inner().inner());
            }
        }

        None
    }

    pub fn fee_transfer_receiver2_balance(&self) -> Option<u64> {
        for internal_balance in self.internal_command_balances() {
            if let mina_rs::InternalCommandBalanceData::FeeTransfer(x) = internal_balance {
                return x
                    .inner()
                    .receiver2_balance
                    .map(|balance| balance.inner().inner().inner());
            }
        }

        None
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

    /// Vec of public keys which send or receive funds in applied commands and coinbase
    pub fn active_public_keys(&self) -> Vec<PublicKey> {
        // block creator and block stake winner
        let mut public_keys: HashSet<PublicKey> =
            HashSet::from([self.block_creator(), self.block_stake_winner()]);

        // coinbase receiver if cooinbase is applied
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

    pub fn with_canonicity(&self, canonicity: Canonicity) -> PrecomputedBlockWithCanonicity {
        PrecomputedBlockWithCanonicity {
            canonicity: Some(canonicity),
            state_hash: self.state_hash.clone(),
            scheduled_time: self.scheduled_time.clone(),
            blockchain_length: self.blockchain_length,
            protocol_state: self.protocol_state.clone(),
            protocol_state_proof: self.protocol_state_proof.clone(),
            staged_ledger_diff: self.staged_ledger_diff.clone(),
            delta_transition_chain_proof: self.delta_transition_chain_proof.clone(),
        }
    }
}

fn add_keys(pks: &mut HashSet<PublicKey>, new_pks: Vec<PublicKey>) {
    for pk in new_pks {
        pks.insert(pk);
    }
}
