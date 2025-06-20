//! V2 mina PCB representation

pub mod precomputed_block;
pub mod protocol_state;
pub mod staged_ledger_diff;
pub mod zkapp;

use crate::{
    base::{
        nonce::Nonce, numeric::Numeric, public_key::PublicKey, scheduled_time::ScheduledTime,
        Balance,
    },
    ledger::{
        account::ReceiptChainHash,
        token::{TokenAddress, TokenSymbol},
    },
};
use log::error;
use protocol_state::ProtocolState;
use serde::{Deserialize, Serialize};
use staged_ledger_diff::StagedLedgerDiff;

// re-export types

pub type AppState = zkapp::app_state::AppState;
pub type ZkappState = zkapp::app_state::ZkappState;
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
    pub scheduled_time: ScheduledTime,
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
    pub balance: Balance,
    pub nonce: Nonce,
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
    pub initial_minimum_balance: Balance,
    pub cliff_time: Numeric<u32>,
    pub vesting_period: Numeric<u32>,
    pub cliff_amount: Balance,
    pub vesting_increment: Balance,
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

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ZkappAccount {
    pub app_state: ZkappState,
    pub action_state: [ActionState; 5],
    pub verification_key: VerificationKey,
    pub proved_state: bool,
    pub zkapp_uri: ZkappUri,
    pub zkapp_version: Numeric<u32>,
    pub last_action_slot: Numeric<u32>,
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

///////////
// check //
///////////

impl crate::base::check::Check for ZkappAccount {
    fn check(&self, other: &Self) -> bool {
        // skip app_state
        // skip proved_state

        if self.proved_state != other.proved_state {
            error!("Mismatching zkapp proved state");
        }

        if self.verification_key != other.verification_key {
            error!(
                "Mismatching zkapp verification keys {:?} {:?}",
                self.verification_key, other.verification_key,
            );
            return true;
        }

        if self.zkapp_uri != other.zkapp_uri {
            error!(
                "Mismatching zkapp URIs {:?} {:?}",
                self.zkapp_uri, other.zkapp_uri,
            );
            return true;
        }

        if self.zkapp_version != other.zkapp_version {
            error!(
                "Mismatching zkapp versions {:?} {:?}",
                self.zkapp_version, other.zkapp_version,
            );
            return true;
        }

        if self.last_action_slot != other.last_action_slot {
            error!(
                "Mismatching zkapp last action slots {:?} {:?}",
                self.last_action_slot, other.last_action_slot,
            );
            return true;
        }

        false
    }
}

impl crate::base::check::Check for Option<ZkappAccount> {
    fn check(&self, other: &Self) -> bool {
        match (self.as_ref(), other.as_ref()) {
            (Some(self_zkapp), Some(zkapp)) => {
                let check = self_zkapp.check(zkapp);
                if check {
                    error!("Mismatching zkapps {:?} {:?}", self_zkapp, zkapp);
                }

                check
            }
            (Some(zkapp), _) | (_, Some(zkapp)) => {
                error!("Mismatching zkapp {:?}", zkapp);
                true
            }
            _ => false,
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

///////////////
// arbitrary //
///////////////

#[cfg(test)]
impl quickcheck::Arbitrary for ZkappUri {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let len = u8::arbitrary(g);

        let mut chars = vec![];
        let alphabet: Vec<_> = ('a'..='z').chain('A'..='Z').chain('0'..='9').collect();

        for _ in 0..len {
            let idx = usize::arbitrary(g) % alphabet.len();

            chars.push(alphabet.get(idx).cloned().unwrap());
        }

        Self(chars.iter().collect())
    }
}
