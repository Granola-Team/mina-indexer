use std::{collections::HashMap, fmt::Error, result::Result};

pub mod account;
pub mod coinbase;
pub mod command;
pub mod diff;
pub mod genesis;
pub mod public_key;

use account::Account;
use diff::LedgerDiff;
use mina_signer::pubkey::PubKeyError;
use public_key::PublicKey;
use serde::{Serialize, Deserialize};

use self::account::{Amount, Nonce};

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

impl Ledger {
    pub fn new() -> Self {
        Ledger {
            accounts: HashMap::new(),
        }
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
    pub fn apply_diff(&mut self, diff: LedgerDiff) -> anyhow::Result<()> {
        diff.public_keys_seen.into_iter().for_each(|public_key| {
            if self.accounts.get(&public_key).is_none() {
                self.accounts
                    .insert(public_key.clone(), Account::empty(public_key));
            }
        });

        let mut success = Ok(());
        diff.account_diffs.into_iter().for_each(|diff| {
            if let Some(account_before) = self.accounts.remove(&diff.public_key().into()) {
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
                    // TODO got this in another branch
                    diff::account::AccountDiff::Delegation(_) => todo!(),
                };
                self.accounts
                    .insert(diff.public_key().into(), account_after);
            } else {
                success = Err(anyhow::Error::new(Error::default()));
            }
        });
        success
    }
}

impl PartialEq for Ledger {
    fn eq(&self, other: &Self) -> bool {
        for pk in self.accounts.keys() {
            if self.accounts.get(pk) != other.accounts.get(pk) {
                println!(
                    "[Ledger.eq mismatch] {pk:?} | {:?} | {:?}",
                    self.accounts.get(pk),
                    other.accounts.get(pk)
                );
                return false;
            }
        }
        for pk in other.accounts.keys() {
            if self.accounts.get(pk) != other.accounts.get(pk) {
                println!(
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
        writeln!(f, "=== Ledger ===")?;
        for account in self.accounts.values() {
            writeln!(f, "{account:?}")?;
        }
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
            .apply_diff(ledger_diff)
            .expect("diff applied successfully");
        ledger
    }

    fn from_diff(ledger_diff: LedgerDiff) -> Self {
        let mut ledger = Ledger::new();
        ledger
            .apply_diff(ledger_diff)
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
