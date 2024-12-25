use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakingPermissions {
    stake: bool,
    edit_state: Permission,
    send: Permission,
    set_delegate: Permission,
    set_permissions: Permission,
    set_verification_key: Permission,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    #[default]
    Signature,
    Proof,
}
