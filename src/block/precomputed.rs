use mina_serialization_types::{
    json::DeltaTransitionChainProofJson,
    protocol_state::{ProtocolState, ProtocolStateJson},
    protocol_state_proof::ProtocolStateProofBase64Json,
    staged_ledger_diff::{
        self, SignedCommandPayloadBody, StagedLedgerDiff, StagedLedgerDiffJson, StakeDelegation,
    },
    v1::{DeltaTransitionChainProof, ProtocolStateProofV1, PublicKeyV1},
};
use serde::{Deserialize, Serialize};

pub struct BlockLogContents {
    pub(crate) state_hash: String,
    pub(crate) blockchain_length: Option<u32>,
    pub(crate) contents: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockLog {
    scheduled_time: String,
    protocol_state: ProtocolStateJson,
    protocol_state_proof: ProtocolStateProofBase64Json,
    staged_ledger_diff: StagedLedgerDiffJson,
    delta_transition_chain_proof: DeltaTransitionChainProofJson,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrecomputedBlock {
    pub state_hash: String,
    pub scheduled_time: String,
    pub protocol_state: ProtocolState,
    pub blockchain_length: Option<u32>,
    pub protocol_state_proof: ProtocolStateProofV1,
    pub staged_ledger_diff: StagedLedgerDiff,
    pub delta_transition_chain_proof: DeltaTransitionChainProof,
}

impl PrecomputedBlock {
    pub fn from_log_contents(log_contents: BlockLogContents) -> serde_json::Result<Self> {
        let state_hash = log_contents.state_hash;
        let blockchain_length = log_contents.blockchain_length;
        let str = String::from_utf8_lossy(&log_contents.contents);
        let BlockLog {
            scheduled_time,
            protocol_state,
            protocol_state_proof,
            staged_ledger_diff,
            delta_transition_chain_proof,
        } = serde_json::from_str::<BlockLog>(&str).unwrap();
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
}

impl PrecomputedBlock {
    pub fn block_public_keys(&self) -> Vec<PublicKeyV1> {
        let mut public_keys = Vec::new();
        let consenesus_state = self
            .protocol_state
            .body
            .clone()
            .inner()
            .inner()
            .consensus_state
            .inner()
            .inner();
        public_keys.append(&mut vec![
            consenesus_state.block_creator,
            consenesus_state.coinbase_receiver,
            consenesus_state.block_stake_winner,
        ]);

        let commands = self
            .staged_ledger_diff
            .diff
            .clone()
            .inner()
            .0
            .inner()
            .inner()
            .commands;
        commands.iter().for_each(|command| {
            let signed_command = match command.clone().inner().data.inner().inner() {
                staged_ledger_diff::UserCommand::SignedCommand(signed_command) => {
                    signed_command.inner().inner()
                }
            };
            public_keys.push(signed_command.signer.0.inner());

            public_keys.push(
                signed_command
                    .payload
                    .clone()
                    .inner()
                    .inner()
                    .common
                    .inner()
                    .inner()
                    .inner()
                    .fee_payer_pk,
            );
            public_keys.append(&mut match signed_command
                .payload
                .inner()
                .inner()
                .body
                .inner()
                .inner()
            {
                SignedCommandPayloadBody::PaymentPayload(payment_payload) => vec![
                    payment_payload.clone().inner().inner().source_pk,
                    payment_payload.inner().inner().receiver_pk,
                ],
                SignedCommandPayloadBody::StakeDelegation(stake_delegation) => {
                    match stake_delegation.inner() {
                        StakeDelegation::SetDelegate {
                            delegator,
                            new_delegate,
                        } => vec![delegator, new_delegate],
                    }
                }
            })
        });

        public_keys
    }
}
