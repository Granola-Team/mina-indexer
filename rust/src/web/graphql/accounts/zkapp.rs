//! GraphQL representation of zkapp account

use crate::{constants::ZKAPP_STATE_FIELD_ELEMENTS_NUM, mina_blocks::v2};
use async_graphql::SimpleObject;

#[derive(SimpleObject)]
pub struct ZkappAccount {
    pub app_state: [String; ZKAPP_STATE_FIELD_ELEMENTS_NUM],
    pub action_state: [String; 5],
    pub verification_key: VerificationKey,
    pub proved_state: bool,
    pub zkapp_uri: String,
    pub zkapp_version: u32,
    pub last_action_slot: u32,
}

#[derive(SimpleObject)]
pub struct VerificationKey {
    pub data: String,
    pub hash: String,
}

impl From<v2::ZkappAccount> for ZkappAccount {
    fn from(value: v2::ZkappAccount) -> Self {
        Self {
            app_state: value.app_state.map(|s| s.0),
            action_state: value.action_state.map(|s| s.0),
            verification_key: value.verification_key.into(),
            proved_state: value.proved_state,
            zkapp_uri: value.zkapp_uri.0,
            zkapp_version: value.zkapp_version.0,
            last_action_slot: value.last_action_slot.0,
        }
    }
}

impl From<v2::VerificationKey> for VerificationKey {
    fn from(value: v2::VerificationKey) -> Self {
        Self {
            data: value.data.0,
            hash: value.hash.0,
        }
    }
}
