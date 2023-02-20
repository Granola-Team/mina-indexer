use std::collections::HashMap;

pub type PublicKey = String;

pub mod account;
pub mod coinbase;
pub mod diff;
pub mod transaction;

use account::Account;
use diff::LedgerDiff;

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
        diff.accounts_created.into_iter().for_each(|account| {
            if self.accounts.get(&account.public_key).is_none() {
                self.accounts.insert(account.public_key.clone(), account);
            }
        });

        let mut success = true; // change this to a Result<(), CustomError> so we can know exactly where this failed
        diff.account_updates.into_iter().for_each(|update| {
            if let Some(account_before) = self.accounts.remove(&update.public_key) {
                let account_after = match &update.update_type {
                    diff::account::UpdateType::Deposit => {
                        Account::from_deposit(account_before, update.amount)
                    }
                    diff::account::UpdateType::Deduction => {
                        Account::from_deduction(account_before, update.amount)
                    }
                };
                self.accounts.insert(update.public_key, account_after);
            } else {
                success = false;
            }
        });
        success
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ledger_apply_diff() {}
}
