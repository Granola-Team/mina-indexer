use account::AccountDiff;

use crate::block_log::PrecomputedBlock;

use super::{coinbase::Coinbase, transaction::Command, PublicKey};

pub mod account;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LedgerDiff {
    pub public_keys_seen: Vec<PublicKey>,
    pub account_diffs: Vec<AccountDiff>,
}

impl LedgerDiff {
    /// the deserialization used by the types used by this function has a lot of room for improvement
    pub fn fom_precomputed_block(precomputed_block: &PrecomputedBlock) -> Self {
        // [A] fallible deserialization function doesn't specify if it fails because it couldn't read a block or because there weren't any of the requested data in a block
        let coinbase = Coinbase::from_precomputed_block(precomputed_block);
        let coinbase_update = coinbase.clone().as_account_diff();

        let commands = Command::from_precomputed_block(precomputed_block); // [A]
        let mut account_diffs_fees: Vec<AccountDiff> =
            AccountDiff::from_block_fees(coinbase.receiver.into(), precomputed_block); // [A]
        let mut account_diffs_transactions = commands
            .iter()
            .cloned()
            .flat_map(AccountDiff::from_command)
            .collect();

        let mut account_diffs = Vec::new();
        account_diffs.append(&mut account_diffs_fees);
        account_diffs.append(&mut account_diffs_transactions);
        account_diffs.push(coinbase_update);

        let public_keys_seen = precomputed_block
            .block_public_keys()
            .into_iter()
            .map(|key| key.into())
            .collect();

        LedgerDiff {
            public_keys_seen,
            account_diffs,
        }
    }

    // potentially make immutable later on
    pub fn append(&mut self, other: Self) {
        other.public_keys_seen.into_iter().for_each(|account| {
            self.public_keys_seen.push(account);
        });

        other.account_diffs.into_iter().for_each(|update| {
            self.account_diffs.push(update);
        });
    }
}
