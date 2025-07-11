use crate::{
    base::check::Check,
    ledger::account::{Account, Permissions},
    mina_blocks::v2,
};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountAccessed {
    pub account: Account,
    // mina ledger index
    pub index: u64,
}

impl AccountAccessed {
    pub fn assert_eq_account(&self, account: &Account, msg: &str) {
        self.account.public_key.log_check(
            &account.public_key,
            &format!(
                "PK mismatch: {}\nGOT: {:?}\nEXPECT: {:?}",
                msg, account.public_key, self.account.public_key,
            ),
        );
        self.account.token.log_check(
            &account.token,
            &format!(
                "Token mismatch: {}\nGOT: {:?}\nEXPECT: {:?}",
                msg, account.token, self.account.token,
            ),
        );
        self.account.balance.log_check(
            &account.balance,
            &format!(
                "Balance mismatch: {}\nGOT: {:?}\nEXPECT: {:?}",
                msg, account.balance, self.account.balance,
            ),
        );
        self.account.nonce.log_check(
            &account.nonce,
            &format!(
                "Nonce mismatch: {}\nGOT: {:?}\nEXPECT: {:?}",
                msg, account.nonce, self.account.nonce,
            ),
        );
        self.account.delegate.log_check(
            &account.delegate,
            &format!(
                "Delegate mismatch: {}\nGOT: {:?}\nEXPECT: {:?}",
                msg, account.delegate, self.account.delegate
            ),
        );
        self.account.zkapp.log_check(
            &account.zkapp,
            &format!(
                "Zkapp mismatch: {}\nGOT: {:?}\nEXPECT: {:?}",
                msg, account.zkapp, self.account.zkapp
            ),
        );
        self.account.token_symbol.log_check(
            &account.token_symbol,
            &format!(
                "Token symbol mismatch: {}\nGOT: {:?}\nEXPECT: {:?}",
                msg, account.token_symbol, self.account.token_symbol
            ),
        );
        self.account.timing.log_check(
            &account.timing,
            &format!(
                "Timing mismatch: {}\nGOT: {:?}\nEXPECT: {:?}",
                msg, account.timing, self.account.timing
            ),
        );
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
            delegate: value.1.delegate.unwrap_or(public_key).into(),
            token: Some(value.1.token_id),
            token_symbol: Some(value.1.token_symbol),
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
