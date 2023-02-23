use std::collections::HashMap;

pub mod account;
pub mod coinbase;
pub mod diff;
pub mod transaction;

use account::Account;
use diff::LedgerDiff;
use mina_serialization_types::v1::PublicKeyV1;

#[derive(Debug, Clone)]
pub struct PublicKey(PublicKeyV1);

impl PartialEq for PublicKey {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for PublicKey {}

impl std::hash::Hash for PublicKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.clone().0.inner().inner().x.hash(state);
    }
}

impl From<PublicKeyV1> for PublicKey {
    fn from(value: PublicKeyV1) -> Self {
        PublicKey(value)
    }
}

impl From<PublicKey> for PublicKeyV1 {
    fn from(value: PublicKey) -> Self {
        value.0
    }
}

#[derive(Default)]
pub struct Ledger {
    accounts: HashMap<PublicKey, Account>,
}

impl Ledger {
    pub fn new() -> Self {
        Ledger {
            accounts: HashMap::new(),
        }
    }

    // should this be a mutable update or immutable?
    pub fn apply_diff(&mut self, diff: LedgerDiff) -> bool {
        diff.public_keys_seen.into_iter().for_each(|public_key| {
            if self.accounts.get(&public_key).is_none() {
                self.accounts
                    .insert(public_key.clone(), Account::empty(public_key.into()));
            }
        });

        let mut success = true; // change this to a Result<(), CustomError> so we can know exactly where this failed
        diff.account_diffs.into_iter().for_each(|diff| {
            if let Some(account_before) = self.accounts.remove(&diff.public_key().into()) {
                let account_after = match &diff {
                    diff::account::AccountDiff::Payment(payment_diff) => {
                        match &payment_diff.update_type {
                            diff::account::UpdateType::Deposit => {
                                Account::from_deposit(account_before, payment_diff.amount)
                            }
                            diff::account::UpdateType::Deduction => {
                                Account::from_deduction(account_before, payment_diff.amount)
                            }
                        }
                    }
                    diff::account::AccountDiff::Delegation(_) => todo!(),
                };
                self.accounts
                    .insert(diff.public_key().into(), account_after);
            } else {
                success = false;
            }
        });
        success
    }
}
