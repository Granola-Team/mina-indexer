use crate::base::numeric::Numeric;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakingPermissions {
    edit_state: Permission,
    send: Permission,
    set_delegate: Permission,
    set_permissions: Permission,
    set_verification_key: Permission,

    #[serde(skip_serializing_if = "Option::is_none")]
    stake: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    access: Option<Permission>,

    #[serde(skip_serializing_if = "Option::is_none")]
    receive: Option<Permission>,

    #[serde(skip_serializing_if = "Option::is_none")]
    set_zkapp_uri: Option<Permission>,

    #[serde(skip_serializing_if = "Option::is_none")]
    edit_action_state: Option<Permission>,

    #[serde(skip_serializing_if = "Option::is_none")]
    set_token_symbol: Option<Permission>,

    #[serde(skip_serializing_if = "Option::is_none")]
    increment_nonce: Option<Permission>,

    #[serde(skip_serializing_if = "Option::is_none")]
    set_voting_for: Option<Permission>,

    #[serde(skip_serializing_if = "Option::is_none")]
    set_timing: Option<Permission>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Permission {
    Auth(PermissionAuth),
    Signature(PermissionSignature),
    Proof(PermissionProof),
    None(PermissionNone),
    Either(PermissionEither),
    Impossible(PermissionImpossible),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionAuth {
    pub auth: PermissionPermission,
    pub txn_version: Numeric<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PermissionSignature {
    #[default]
    Signature,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PermissionProof {
    #[default]
    Proof,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PermissionNone {
    #[default]
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PermissionEither {
    #[default]
    Either,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PermissionImpossible {
    #[default]
    Impossible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PermissionPermission {
    #[default]
    Signature,
    Proof,
    None,
    Either,
    Impossible,
}

impl std::default::Default for Permission {
    fn default() -> Self {
        Self::Signature(PermissionSignature::Signature)
    }
}
