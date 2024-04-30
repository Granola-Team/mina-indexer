pub mod precomputed_block;
pub mod protocol_state;
pub mod staged_ledger_diff;

use crate::{
    block::BlockHash,
    ledger::{public_key::PublicKey, staking::ReceiptChainHash},
    mina_blocks::common::*,
};
use protocol_state::ProtocolState;
use serde::{Deserialize, Serialize};
use staged_ledger_diff::StagedLedgerDiff;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecomputedBlockV2 {
    pub version: u32,
    pub data: PrecomputedBlockDataV2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecomputedBlockDataV2 {
    #[serde(deserialize_with = "from_str")]
    pub scheduled_time: u64,

    pub tokens_used: Vec<(String, Option<String>)>,
    pub protocol_version: ProtocolVersion,
    pub proposed_protocol_version: Option<ProtocolVersion>,
    pub protocol_state: ProtocolState,
    pub staged_ledger_diff: StagedLedgerDiff,
    pub accounts_created: Vec<(u64, PublicKey)>,
    pub accounts_accessed: Vec<(u64, AccountAccessed)>,

    #[serde(skip_deserializing)]
    pub delta_transition_chain_proof: serde_json::Value,

    #[serde(skip_deserializing)]
    pub protocol_state_proof: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountAccessed {
    #[serde(deserialize_with = "from_str")]
    pub public_key: PublicKey,

    #[serde(deserialize_with = "from_str")]
    pub balance: u64,

    #[serde(deserialize_with = "from_str")]
    pub nonce: u32,

    #[serde(deserialize_with = "from_str")]
    pub receipt_chain_hash: ReceiptChainHash,

    #[serde(deserialize_with = "from_str")]
    pub delegate: PublicKey,

    #[serde(deserialize_with = "from_str")]
    pub voting_for: BlockHash,

    pub token_id: String,
    pub token_symbol: String,
    pub permissions: Permissions,
    pub timing: (Timing,),

    // TODO
    pub zkapp: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Timing {
    Untimed,
    Timed {
        #[serde(deserialize_with = "from_str")]
        initial_minimum_balance: u64,

        #[serde(deserialize_with = "from_str")]
        cliff_time: u64,

        #[serde(deserialize_with = "from_str")]
        cliff_amount: u64,

        #[serde(deserialize_with = "from_str")]
        vesting_period: u64,

        #[serde(deserialize_with = "from_str")]
        vesting_increment: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permissions {
    pub edit_state: (Permission,),
    pub access: (Permission,),
    pub send: (Permission,),
    pub receive: (Permission,),
    pub set_delegate: (Permission,),
    pub set_permissions: (Permission,),
    pub set_verification_key: ((Permission, SetVerificationKey),),
    pub set_zkapp_uri: (Permission,),
    pub edit_action_state: (Permission,),
    pub set_token_symbol: (Permission,),
    pub increment_nonce: (Permission,),
    pub set_voting_for: (Permission,),
    pub set_timing: (Permission,),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetVerificationKey(pub u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission(pub [String; 1]);

/// See https://github.com/MinaProtocol/mina/blob/berkeley/src/lib/mina_base/permissions.mli

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionKind {
    None,
    Either,
    Proof,
    Signature,
    Impossible,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeltaTransitionChainProof {
    Array(Vec<BlockHash>),
    BlockHash(BlockHash),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub transaction: u32,
    pub network: u32,
    pub patch: u32,
}
