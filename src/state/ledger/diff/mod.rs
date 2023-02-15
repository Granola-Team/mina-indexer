use crate::block_log::{BlockLog, public_keys_seen, get_block_commands};

use account::AccountDiff;

use super::{account::Account, transaction::Transaction, coinbase::Coinbase};

pub mod account;

pub struct LedgerDiff {
    pub accounts_created: Vec<Account>,
    pub account_updates: Vec<AccountDiff>,
}

impl LedgerDiff {
    /// the deserialization used by the types used by this function has a lot of room for improvement
    pub fn fom_block_log(block_log: BlockLog) -> Option<Self> {
        let coinbase = Coinbase::from_block_log(&block_log)?;
        let coinbase_update = coinbase.clone().as_account_diff();

        let commands = get_block_commands(&block_log)?;

        let transactions = Transaction::from_commands(&commands);
        let mut account_updates_fees: Vec<AccountDiff> = AccountDiff::from_commands_fees(coinbase.receiver.clone(), &commands);
        let mut account_updates_transactions = transactions
            .iter()
            .cloned()
            .map(|transaction| AccountDiff::from_transaction(transaction))
            .flatten()
            .collect();

        let mut account_updates = Vec::new();
        account_updates.append(&mut account_updates_fees);
        account_updates.append(&mut account_updates_transactions);
        account_updates.push(coinbase_update);

        let accounts_created = public_keys_seen(&block_log)
            .into_iter()
            .map(|public_key| Account::empty(public_key))
            .collect();

        Some(LedgerDiff {
            accounts_created,
            account_updates,
        })
    }

    // potentially make immutable later on
    pub fn append(&mut self, other: Self) {
        other.accounts_created.into_iter().for_each(|account| {
            self.accounts_created.push(account);
        });

        other.account_updates.into_iter().for_each(|update| {
            self.account_updates.push(update);
        });
    }
}
