pub mod account;
pub mod coinbase;
pub mod command;
pub mod diff;
pub mod genesis;
pub mod public_key;
pub mod store;
pub mod user_commands;

use crate::{block::precomputed::PrecomputedBlock, state::ledger::user_commands::UserCommandType};

use self::{
    account::{Amount, Nonce},
    user_commands::{BalanceUpdate, UserCommand},
};
use account::Account;
use diff::LedgerDiff;
use mina_signer::pubkey::PubKeyError;
use public_key::PublicKey;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, result::Result};
use tracing::debug;

impl ExtendWithLedgerDiff for LedgerMock {
    fn extend_with_diff(self, _ledger_diff: LedgerDiff) -> Self {
        LedgerMock {}
    }

    fn from_diff(_ledger_diff: LedgerDiff) -> Self {
        LedgerMock {}
    }
}

#[derive(Default, Clone, Debug)]
pub struct LedgerMock {}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Ledger {
    pub accounts: HashMap<PublicKey, Account>,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct NonGenesisLedger {
    pub ledger: Ledger,
}

#[derive(Debug, Clone)]
pub enum LedgerError {
    AccountNotFound,
    InvalidDelegation,
}

impl Ledger {
    pub fn new() -> Self {
        Ledger {
            accounts: HashMap::new(),
        }
    }

    pub fn apply_delegation(&mut self, delegator: PublicKey, new_delegate: PublicKey) {
        if let Some(account) = self.accounts.get_mut(&delegator) {
            if let Some(old_delegate) = &account.delegate {
                if old_delegate == &new_delegate {
                    return;
                }
            }

            account.delegate = Some(new_delegate);
        }
    }

    pub fn apply_balance_update(&mut self, balance_update: BalanceUpdate, nonce: Option<i32>) {
        if let Some(account) = self.accounts.get_mut(&balance_update.public_key) {
            let nonce = if let Some(nonce) = nonce {
                Nonce(nonce as u32 + 1)
            } else {
                account.nonce
            };
            account.balance = Amount(balance_update.balance);
            account.nonce = nonce;
        } else {
            let new_account = Account {
                public_key: balance_update.public_key.clone(),
                balance: Amount(balance_update.balance),
                nonce: if let Some(nonce) = nonce {
                    Nonce(nonce as u32)
                } else {
                    Nonce(0)
                },
                delegate: None,
            };
            self.accounts.insert(balance_update.public_key, new_account);
        }
    }

    pub fn apply_post_balances(&mut self, precomputed_block: &PrecomputedBlock) {
        UserCommand::from_precomputed(precomputed_block)
            .into_iter()
            .for_each(|user_command| {
                if UserCommandType::Delegation == user_command.command_type {
                    self.apply_delegation(
                        user_command.source.public_key.clone(),
                        user_command.receiver.public_key.clone(),
                    );
                }
                self.apply_balance_update(user_command.fee_payer, None);
                self.apply_balance_update(user_command.receiver, None);
                self.apply_balance_update(user_command.source, Some(user_command.source_nonce));
            });
    }

    pub fn from(value: Vec<(&str, u64, Option<u32>, Option<&str>)>) -> Result<Self, PubKeyError> {
        let mut ledger = Ledger::new();
        for (pubkey, balance, nonce, delgation) in value {
            match PublicKey::from_address(pubkey) {
                Ok(pk) => {
                    if let Some(delegate) = delgation {
                        match PublicKey::from_address(delegate) {
                            Ok(delegate) => {
                                ledger.accounts.insert(
                                    pk.clone(),
                                    Account {
                                        public_key: pk,
                                        balance: Amount(balance),
                                        nonce: Nonce(nonce.unwrap_or_default()),
                                        delegate: Some(delegate),
                                    },
                                );
                            }
                            Err(err) => return Err(err),
                        }
                    } else {
                        let acct = Account {
                            public_key: pk.clone(),
                            balance: Amount(balance),
                            nonce: Nonce(nonce.unwrap_or_default()),
                            delegate: None,
                        };
                        ledger.accounts.insert(pk, acct);
                    }
                }
                Err(err) => return Err(err),
            }
        }
        Ok(ledger)
    }

    // should this be a mutable update or immutable?
    pub fn apply_diff(&mut self, diff: &LedgerDiff) -> anyhow::Result<()> {
        let diff = diff.clone();

        diff.public_keys_seen.into_iter().for_each(|public_key| {
            if self.accounts.get(&public_key).is_none() {
                self.accounts
                    .insert(public_key.clone(), Account::empty(public_key));
            }
        });

        for diff in diff.account_diffs {
            match self.accounts.remove(&diff.public_key()) {
                Some(account_before) => {
                    let account_after = match &diff {
                        diff::account::AccountDiff::Payment(payment_diff) => {
                            match &payment_diff.update_type {
                                diff::account::UpdateType::Deposit => {
                                    Account::from_deposit(account_before, payment_diff.amount)
                                }
                                diff::account::UpdateType::Deduction => {
                                    match Account::from_deduction(
                                        account_before.clone(),
                                        payment_diff.amount,
                                    ) {
                                        Some(account) => account,
                                        None => account_before,
                                    }
                                }
                            }
                        }
                        diff::account::AccountDiff::Delegation(delegation_diff) => {
                            assert_eq!(account_before.public_key, delegation_diff.delegator);
                            Account::from_delegation(
                                account_before,
                                delegation_diff.delegate.clone(),
                            )
                        }
                    };

                    self.accounts.insert(diff.public_key(), account_after);
                }
                None => {
                    let error = match diff {
                        diff::account::AccountDiff::Payment(_) => LedgerError::AccountNotFound,
                        diff::account::AccountDiff::Delegation(_) => LedgerError::InvalidDelegation,
                    };

                    return Err(error.into());
                }
            }
        }
        Ok(())
    }
}

impl PartialEq for Ledger {
    fn eq(&self, other: &Self) -> bool {
        for pk in self.accounts.keys() {
            if self.accounts.get(pk) != other.accounts.get(pk) {
                debug!(
                    "[Ledger.eq mismatch] {pk:?} | {:?} | {:?}",
                    self.accounts.get(pk),
                    other.accounts.get(pk)
                );
                return false;
            }
        }
        for pk in other.accounts.keys() {
            if self.accounts.get(pk) != other.accounts.get(pk) {
                debug!(
                    "[Ledger.eq mismatch] {pk:?} | {:?} | {:?}",
                    self.accounts.get(pk),
                    other.accounts.get(pk)
                );
                return false;
            }
        }
        true
    }
}

impl Eq for Ledger {}

impl std::fmt::Debug for Ledger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for account in self.accounts.values() {
            write!(f, "{account:?}")?;
        }
        writeln!(f)?;
        Ok(())
    }
}

pub trait ExtendWithLedgerDiff {
    fn extend_with_diff(self, ledger_diff: LedgerDiff) -> Self;
    fn from_diff(ledger_diff: LedgerDiff) -> Self;
}

impl ExtendWithLedgerDiff for Ledger {
    fn extend_with_diff(self, ledger_diff: LedgerDiff) -> Self {
        let mut ledger = self;
        ledger
            .apply_diff(&ledger_diff)
            .expect("diff applied successfully");
        ledger
    }

    fn from_diff(ledger_diff: LedgerDiff) -> Self {
        let mut ledger = Ledger::new();
        ledger
            .apply_diff(&ledger_diff)
            .expect("diff applied successfully");
        ledger
    }
}

impl ExtendWithLedgerDiff for LedgerDiff {
    fn extend_with_diff(self, ledger_diff: LedgerDiff) -> Self {
        let mut to_extend = self;
        to_extend.append(ledger_diff);
        to_extend
    }

    fn from_diff(ledger_diff: LedgerDiff) -> Self {
        ledger_diff
    }
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LedgerError::AccountNotFound => write!(f, "Account not found in ledger: payment error"),
            LedgerError::InvalidDelegation => {
                write!(f, "Invalid data or parameters: delegation error")
            }
        }
    }
}

impl std::error::Error for LedgerError {}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::state::ledger::{account::Amount, diff::account::DelegationDiff};

    use super::{
        account::Account,
        diff::{
            account::{AccountDiff, PaymentDiff, UpdateType},
            LedgerDiff,
        },
        public_key::PublicKey,
        Ledger,
    };

    #[test]
    fn apply_diff_payment() {
        let public_key =
            PublicKey::from_address("B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy")
                .expect("public key creation");
        let account = Account::empty(public_key.clone());
        let mut accounts = HashMap::new();
        accounts.insert(public_key.clone(), account);
        let mut ledger = Ledger { accounts };

        let ledger_diff = LedgerDiff {
            public_keys_seen: vec![],
            account_diffs: vec![AccountDiff::Payment(PaymentDiff {
                public_key: public_key.clone(),
                amount: 1,
                update_type: UpdateType::Deposit,
            })],
        };

        ledger
            .apply_diff(&ledger_diff)
            .expect("ledger diff application");

        let account_after = ledger.accounts.get(&public_key).expect("account get");

        assert_eq!(account_after.balance, Amount(1));
    }

    #[test]
    fn apply_diff_delegation() {
        let public_key =
            PublicKey::from_address("B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy")
                .expect("public key creation");
        let delegate_key =
            PublicKey::from_address("B62qmMypEDCchUgPD6RU99gVKXJcY46urKdjbFmG5cYtaVpfKysXTz6")
                .expect("delegate public key creation");
        let account = Account::empty(public_key.clone());
        let mut accounts = HashMap::new();
        accounts.insert(public_key.clone(), account);
        let mut ledger = Ledger { accounts };

        let ledger_diff = LedgerDiff {
            public_keys_seen: vec![],
            account_diffs: vec![AccountDiff::Delegation(DelegationDiff {
                delegator: public_key.clone(),
                delegate: delegate_key.clone(),
            })],
        };

        ledger
            .apply_diff(&ledger_diff)
            .expect("ledger diff application");

        let account_after = ledger.accounts.get(&public_key).expect("account get");

        assert_eq!(account_after.delegate, Some(delegate_key));
    }
}
