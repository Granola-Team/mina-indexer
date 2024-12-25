use crate::{
    ledger::account::{Account, Permissions},
    mina_blocks::v2,
};
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct AccountAccessed {
    pub account: Account,
    // mina ledger index
    pub index: u64,
}

impl From<(u64, v2::AccountAccessed)> for AccountAccessed {
    fn from(value: (u64, v2::AccountAccessed)) -> Self {
        let index = value.0;
        let public_key = value.1.public_key.to_owned();

        let timing = match value.1.timing {
            v2::AccountAccessedTiming::Untimed(_) => None,
            v2::AccountAccessedTiming::Timed((_, timing)) => Some(timing.into()),
        };

        let account = Account {
            public_key: public_key.to_owned(),
            balance: value.1.balance.into(),
            nonce: Some(value.1.nonce.into()),
            delegate: value.1.delegate.unwrap_or(public_key),
            genesis_account: false,
            token: Some(value.1.token_id),
            receipt_chain_hash: Some(value.1.receipt_chain_hash),
            voting_for: Some(value.1.voting_for),
            permissions: Some(value.1.permissions.into()),
            timing,
            zkapp: value.1.zkapp,
            username: None,
        };

        Self { account, index }
    }
}

impl From<v2::Permissions> for Permissions {
    fn from(value: v2::Permissions) -> Self {
        Self {
            edit_state: value.edit_state.0.into(),
            access: value.access.0.into(),
            send: value.send.0.into(),
            receive: value.receive.0.into(),
            set_delegate: value.set_delegate.0.into(),
            set_permissions: value.set_permissions.0.into(),
            set_verification_key: (
                value.set_verification_key.0 .0.into(),
                value.set_verification_key.1,
            ),
            set_zkapp_uri: value.set_zkapp_uri.0.into(),
            edit_action_state: value.edit_action_state.0.into(),
            set_token_symbol: value.set_token_symbol.0.into(),
            increment_nonce: value.increment_nonce.0.into(),
            set_voting_for: value.set_voting_for.0.into(),
            set_timing: value.set_timing.0.into(),
        }
    }
}
