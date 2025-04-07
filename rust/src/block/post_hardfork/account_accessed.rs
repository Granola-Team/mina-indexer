use crate::{
    ledger::account::{Account, Permissions},
    mina_blocks::v2,
};
use log::warn;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountAccessed {
    pub account: Account,
    // mina ledger index
    pub index: u64,
}

impl AccountAccessed {
    pub fn assert_eq_account(&self, account: &Account, msg: &str) {
        assert_eq(
            &self.account.public_key,
            &account.public_key,
            format!(
                "PK mismatch: {}\nGOT: {:?}\nEXPECT: {:?}",
                msg, self.account.public_key, account.public_key
            ),
        );
        assert_eq(
            &self.account.token,
            &account.token,
            format!(
                "Token mismatch: {}\nGOT: {:?}\nEXPECT: {:?}",
                msg, self.account.token, account.token
            ),
        );
        assert_eq(
            &self.account.balance,
            &account.balance,
            format!(
                "Balance mismatch: {}\nGOT: {:?}\nEXPECT: {:?}",
                msg, self.account.balance, account.balance
            ),
        );
        assert_eq(
            &self.account.nonce.unwrap_or_default(),
            &account.nonce.unwrap_or_default(),
            format!(
                "Nonce mismatch: {}\nGOT: {:?}\nEXPECT: {:?}",
                msg,
                self.account.nonce.unwrap_or_default(),
                account.nonce.unwrap_or_default()
            ),
        );
        assert_eq(
            &self.account.delegate,
            &account.delegate,
            format!(
                "Delegate mismatch: {}\nGOT: {:?}\nEXPECT: {:?}",
                msg, self.account.delegate, account.delegate
            ),
        );
        assert_eq(
            &self.account.zkapp,
            &account.zkapp,
            format!(
                "Zkapp mismatch: {}\nGOT: {:?}\nEXPECT: {:?}",
                msg, self.account.zkapp, account.zkapp,
            ),
        );
        assert_eq(
            &self.account.token_symbol.clone().unwrap_or_default(),
            &account.token_symbol.clone().unwrap_or_default(),
            format!(
                "Token symbol mismatch: {}\nGOT: {:?}\nEXPECT: {:?}",
                msg,
                self.account.token_symbol.clone().unwrap_or_default(),
                account.token_symbol.clone().unwrap_or_default()
            ),
        );
        // assert_eq(
        //     &self.account.timing,
        //     &account.timing,
        //     format!("Timing mismatch: {}", msg),
        // );
        // assert_eq(
        //     &self.account.permissions,
        //     &account.permissions,
        //     format!("Permissions mismatch: {}", msg),
        // )
    }
}

fn assert_eq<T: PartialEq>(x: &T, y: &T, msg: String) {
    if x != y {
        warn!("{msg}")
    }
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
            balance: value.1.balance.0.into(),
            nonce: Some(value.1.nonce),
            delegate: value.1.delegate.unwrap_or(public_key),
            token: Some(value.1.token_id),
            token_symbol: Some(value.1.token_symbol),
            receipt_chain_hash: Some(value.1.receipt_chain_hash),
            voting_for: Some(value.1.voting_for.into()),
            permissions: Some(value.1.permissions.into()),
            timing,
            zkapp: value.1.zkapp,
            ..Default::default()
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
