pub mod precomputed_block;
pub mod protocol_state;
pub mod staged_ledger_diff;

use crate::{
    block::BlockHash,
    constants::ZKAPP_STATE_FIELD_ELEMENTS_NUM,
    ledger::{
        account::ReceiptChainHash,
        public_key::PublicKey,
        token::{symbol::TokenSymbol, TokenAddress},
    },
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

    pub protocol_version: ProtocolVersion,
    pub proposed_protocol_version: Option<ProtocolVersion>,
    pub protocol_state: ProtocolState,
    pub staged_ledger_diff: StagedLedgerDiff,

    /// values: ((pk, token), u64)
    pub accounts_created: Vec<AccountCreated>,
    pub accounts_accessed: Vec<(u64, AccountAccessed)>,
    pub tokens_used: Vec<TokenUsed>,

    #[serde(skip_deserializing)]
    pub delta_transition_chain_proof: (String, [String; 0]),

    #[serde(skip_deserializing)]
    pub protocol_state_proof: serde_json::Value,
}

// aliases

pub type TokenUsed = (TokenAddress, Option<(PublicKey, TokenAddress)>);
pub type AccountCreated = ((PublicKey, TokenAddress), String);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountAccessed {
    #[serde(deserialize_with = "from_str")]
    pub public_key: PublicKey,

    #[serde(deserialize_with = "from_str")]
    pub balance: u64,

    #[serde(deserialize_with = "from_str")]
    pub nonce: u32,

    #[serde(deserialize_with = "from_str")]
    pub receipt_chain_hash: ReceiptChainHash,

    #[serde(deserialize_with = "from_str_opt")]
    pub delegate: Option<PublicKey>,

    #[serde(deserialize_with = "from_str")]
    pub voting_for: BlockHash,

    #[serde(deserialize_with = "from_str")]
    pub token_id: TokenAddress,

    #[serde(deserialize_with = "from_str")]
    pub token_symbol: TokenSymbol,

    pub permissions: Permissions,
    pub timing: AccountAccessedTiming,
    pub zkapp: Option<ZkappAccount>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AccountAccessedTiming {
    Untimed((TimingKind,)),
    Timed((TimingKind, Timing)),
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timing {
    #[serde(deserialize_with = "from_str")]
    pub initial_minimum_balance: u64,

    #[serde(deserialize_with = "from_str")]
    pub cliff_time: u32,

    #[serde(deserialize_with = "from_str")]
    pub cliff_amount: u64,

    #[serde(deserialize_with = "from_str")]
    pub vesting_period: u32,

    #[serde(deserialize_with = "from_str")]
    pub vesting_increment: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimingKind {
    Timed,
    Untimed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Permissions {
    pub edit_state: Permission,
    pub access: Permission,
    pub send: Permission,
    pub receive: Permission,
    pub set_delegate: Permission,
    pub set_permissions: Permission,
    pub set_verification_key: (Permission, String),
    pub set_zkapp_uri: Permission,
    pub edit_action_state: Permission,
    pub set_token_symbol: Permission,
    pub increment_nonce: Permission,
    pub set_voting_for: Permission,
    pub set_timing: Permission,
}

pub type Permission = (PermissionKind,);

/// See https://github.com/MinaProtocol/mina/blob/berkeley/src/lib/mina_base/permissions.mli

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PermissionKind {
    None,
    Either,
    Proof,
    Signature,
    Impossible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZkappAccount {
    pub app_state: [String; ZKAPP_STATE_FIELD_ELEMENTS_NUM], // 32 bytes each
    pub action_state: [String; 5],                           // 32 bytes each
    pub verification_key: VerificationKey,
    pub proved_state: bool,
    pub zkapp_uri: String,

    #[serde(deserialize_with = "from_str")]
    pub zkapp_version: u32,

    #[serde(deserialize_with = "from_str")]
    pub last_action_slot: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationKey {
    pub data: String,
    pub hash: VerificationKeyHash,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationKeyHash(pub String);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub transaction: u32,
    pub network: u32,
    pub patch: u32,
}