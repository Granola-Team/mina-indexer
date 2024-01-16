use crate::{
    block::{get_blockchain_length, get_state_hash, is_valid_block_file, BlockHash},
    command::signed,
    command::{signed::SignedCommand, PaymentPayload, UserCommandWithStatus},
    ledger::{coinbase::Coinbase, public_key::PublicKey},
    MAINNET_GENESIS_TIMESTAMP,
};
use anyhow::anyhow;
use mina_serialization_types::{
    json::DeltaTransitionChainProofJson,
    protocol_state::{ProtocolState, ProtocolStateJson},
    protocol_state_proof::ProtocolStateProofBase64Json,
    staged_ledger_diff as mina_rs,
    v1::{DeltaTransitionChainProof, ProtocolStateProofV1},
};
use serde::{Deserialize, Serialize};
use std::path::Path;

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
    pub protocol_state: ProtocolState,
    pub blockchain_length: u32,
    pub protocol_state_proof: ProtocolStateProofV1,
    pub staged_ledger_diff: mina_rs::StagedLedgerDiff,
    pub delta_transition_chain_proof: DeltaTransitionChainProof,
}

impl PrecomputedBlock {
    pub fn from_file_contents(log_contents: BlockFileContents) -> serde_json::Result<Self> {
        let state_hash = log_contents.state_hash;
        let str = String::from_utf8_lossy(&log_contents.contents);
        let BlockFile {
            scheduled_time,
            protocol_state,
            protocol_state_proof,
            staged_ledger_diff,
            delta_transition_chain_proof,
        } = serde_json::from_str::<BlockFile>(&str).unwrap();
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

    pub fn consensus_state(&self) -> mina_serialization_types::consensus_state::ConsensusState {
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

    pub fn completed_works(
        &self,
    ) -> Vec<mina_serialization_types::snark_work::TransactionSnarkWork> {
        self.staged_ledger_pre_diff()
            .completed_works
            .iter()
            .map(|x| x.t.clone())
            .collect()
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

    pub fn block_public_keys(&self) -> Vec<PublicKey> {
        let mut public_keys: Vec<PublicKey> = vec![];
        let consenesus_state = self.consensus_state();

        public_keys.append(&mut vec![
            consenesus_state.block_creator.into(),
            consenesus_state.coinbase_receiver.into(),
            consenesus_state.block_stake_winner.into(),
        ]);

        let commands = self.commands();
        commands.iter().for_each(|command| {
            let signed_command = match command.clone().data() {
                mina_rs::UserCommand::SignedCommand(signed_command) => {
                    SignedCommand(signed_command)
                }
            };
            public_keys.push(signed_command.signer());
            public_keys.push(signed_command.fee_payer_pk());
            public_keys.append(&mut match signed_command.payload_body() {
                mina_rs::SignedCommandPayloadBody::PaymentPayload(payment_payload) => vec![
                    PaymentPayload(payment_payload.clone()).source_pk(),
                    PaymentPayload(payment_payload).receiver_pk(),
                ],
                mina_rs::SignedCommandPayloadBody::StakeDelegation(stake_delegation) => {
                    match stake_delegation.inner() {
                        mina_rs::StakeDelegation::SetDelegate {
                            delegator,
                            new_delegate,
                        } => vec![delegator.into(), new_delegate.into()],
                    }
                }
            })
        });

        public_keys
    }

    pub fn active_block_public_keys(&self) -> Vec<PublicKey> {
        let mut public_keys: Vec<PublicKey> = vec![];
        let consenesus_state = self.consensus_state();

        public_keys.append(&mut vec![
            consenesus_state.block_creator.into(),
            consenesus_state.block_stake_winner.into(),
        ]);

        if Coinbase::from_precomputed(self).is_coinbase_applied() {
            public_keys.push(consenesus_state.coinbase_receiver.into());
        }

        let commands = self.commands();
        commands
            .iter()
            .filter(|cmd| cmd.is_applied())
            .for_each(|command| {
                let signed_command = match command.clone().data() {
                    mina_rs::UserCommand::SignedCommand(signed_command) => {
                        SignedCommand(signed_command)
                    }
                };
                public_keys.push(signed_command.signer());
                public_keys.push(signed_command.fee_payer_pk());
                public_keys.append(&mut match signed_command.payload_body() {
                    mina_rs::SignedCommandPayloadBody::PaymentPayload(payment_payload) => vec![
                        PaymentPayload(payment_payload.clone()).source_pk(),
                        PaymentPayload(payment_payload).receiver_pk(),
                    ],
                    mina_rs::SignedCommandPayloadBody::StakeDelegation(stake_delegation) => {
                        match stake_delegation.inner() {
                            mina_rs::StakeDelegation::SetDelegate {
                                delegator,
                                new_delegate,
                            } => vec![delegator.into(), new_delegate.into()],
                        }
                    }
                })
            });

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
        signed::SignedCommand::from_precomputed(self)
            .iter()
            .map(|cmd| cmd.hash_signed_command().unwrap())
            .collect()
    }
}
