use crate::block_log::{get_block_commands, public_keys_seen, BlockLog};

use account::AccountDiff;

use super::{account::Account, coinbase::Coinbase, transaction::Transaction};

pub mod account;

pub struct LedgerDiff {
    pub accounts_created: Vec<Account>,
    pub account_updates: Vec<AccountDiff>,
}

impl LedgerDiff {
    /// the deserialization used by the types used by this function has a lot of room for improvement
    pub fn fom_block_log(block_log: BlockLog) -> Option<Self> {
        // [A] fallible deserialization function doesn't specify if it fails because it couldn't read a block or because there weren't any of the requested data in a block
        let coinbase = Coinbase::from_block_log(&block_log)?;
        let coinbase_update = coinbase.clone().as_account_diff();

        let commands = get_block_commands(&block_log)?; // [A]

        let transactions = Transaction::from_commands(&commands); // [A]
        let mut account_diffs_fees: Vec<AccountDiff> =
            AccountDiff::from_commands_fees(coinbase.receiver.clone(), &commands); // [A]
        let mut account_diffs_transactions = transactions
            .iter()
            .cloned()
            .map(|transaction| AccountDiff::from_transaction(transaction))
            .flatten()
            .collect();

        let mut account_diffs = Vec::new();
        account_diffs.append(&mut account_diffs_fees);
        account_diffs.append(&mut account_diffs_transactions);
        account_diffs.push(coinbase_update);

        let accounts_created = public_keys_seen(&block_log) // [A]
            .into_iter()
            .map(|public_key| Account::empty(public_key))
            .collect();

        Some(LedgerDiff {
            accounts_created,
            account_updates: account_diffs,
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
