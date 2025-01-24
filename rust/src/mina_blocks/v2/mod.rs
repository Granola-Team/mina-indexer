pub mod precomputed_block;
pub mod protocol_state;
pub mod staged_ledger_diff;

mod zkapp;

use crate::{
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

// re-export types

pub type AppState = zkapp::app_state::AppState;
pub type ActionState = zkapp::action_state::ActionState;
pub type ZkappEvent = zkapp::event::ZkappEvent;
pub type VerificationKey = zkapp::verification_key::VerificationKey;

// v2 PCB (de)serialization

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecomputedBlockV2 {
    pub version: u32,
    pub data: PrecomputedBlockDataV2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecomputedBlockDataV2 {
    #[serde(serialize_with = "to_str")]
    #[serde(deserialize_with = "from_str")]
    pub scheduled_time: u64,

    pub protocol_version: ProtocolVersion,
    pub proposed_protocol_version: Option<ProtocolVersion>,
    pub protocol_state: ProtocolState,
    pub staged_ledger_diff: StagedLedgerDiff,
    pub accounts_created: Vec<AccountCreated>,
    pub accounts_accessed: Vec<(u64, AccountAccessed)>,
    pub tokens_used: Vec<TokenUsed>,

    #[serde(skip_deserializing)]
    pub delta_transition_chain_proof: (String, [String; 0]),

    #[serde(skip_deserializing)]
    pub protocol_state_proof: serde_json::Value,
}

// aliases

/// values: `((pk, token), account creation fee u64 as string)`
pub type AccountCreated = ((PublicKey, TokenAddress), String);
pub type TokenUsed = (TokenAddress, Option<(PublicKey, TokenAddress)>);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountAccessed {
    #[serde(serialize_with = "to_str")]
    #[serde(deserialize_with = "from_str")]
    pub balance: u64,

    #[serde(serialize_with = "to_str")]
    #[serde(deserialize_with = "from_str")]
    pub nonce: u32,

    pub public_key: PublicKey,
    pub receipt_chain_hash: ReceiptChainHash,
    pub delegate: Option<PublicKey>,
    pub token_id: TokenAddress,
    pub token_symbol: TokenSymbol,
    pub voting_for: String,
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

#[derive(Default, Clone, Debug, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
pub struct Timing {
    #[serde(serialize_with = "to_str")]
    #[serde(deserialize_with = "from_str")]
    pub initial_minimum_balance: u64,

    #[serde(serialize_with = "to_str")]
    #[serde(deserialize_with = "from_str")]
    pub cliff_time: u32,

    #[serde(serialize_with = "to_str")]
    #[serde(deserialize_with = "from_str")]
    pub cliff_amount: u64,

    #[serde(serialize_with = "to_str")]
    #[serde(deserialize_with = "from_str")]
    pub vesting_period: u32,

    #[serde(serialize_with = "to_str")]
    #[serde(deserialize_with = "from_str")]
    pub vesting_increment: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimingKind {
    Timed,
    Untimed,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum PermissionKind {
    None,
    Either,
    Proof,
    Signature,
    Impossible,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZkappAccount {
    pub app_state: [AppState; ZKAPP_STATE_FIELD_ELEMENTS_NUM],
    pub action_state: [ActionState; 5],
    pub verification_key: VerificationKey,
    pub proved_state: bool,
    pub zkapp_uri: ZkappUri,

    #[serde(serialize_with = "to_str")]
    #[serde(deserialize_with = "from_str")]
    pub zkapp_version: u32,

    #[serde(serialize_with = "to_str")]
    #[serde(deserialize_with = "from_str")]
    pub last_action_slot: u32,
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappUri(pub String);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub transaction: u32,
    pub network: u32,
    pub patch: u32,
}

///////////
// impls //
///////////

impl ZkappAccount {
    pub fn from_proved_state(proved_state: bool) -> Self {
        Self {
            proved_state,
            ..Default::default()
        }
    }
}

/////////////////
// conversions //
/////////////////

impl<T> From<T> for ZkappUri
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}
