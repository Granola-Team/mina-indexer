use account::AccountDiff;
use serde::{Deserialize, Serialize};

use crate::block::precomputed::PrecomputedBlock;

use super::{coinbase::Coinbase, command::Command, PublicKey};

pub mod account;

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LedgerDiff {
    pub public_keys_seen: Vec<PublicKey>,
    pub account_diffs: Vec<AccountDiff>,
}

impl LedgerDiff {
    /// the deserialization used by the types used by this function has a lot of room for improvement
    pub fn from_precomputed_block(precomputed_block: &PrecomputedBlock) -> Self {
        // [A] fallible deserialization function doesn't specify if it fails because it couldn't read a block or because there weren't any of the requested data in a block
        let coinbase = Coinbase::from_precomputed_block(precomputed_block);
        let coinbase_update = coinbase.clone().as_account_diff();

        let commands = Command::from_precomputed_block(precomputed_block); // [A]
        let mut account_diffs_fees: Vec<AccountDiff> =
            AccountDiff::from_block_fees(coinbase.receiver, precomputed_block); // [A]
        let mut account_diffs_transactions = commands
            .iter()
            .cloned()
            .flat_map(AccountDiff::from_command)
            .collect();

        let mut account_diffs = Vec::new();
        account_diffs.append(&mut account_diffs_fees);
        account_diffs.append(&mut account_diffs_transactions);
        account_diffs.push(coinbase_update);

        let public_keys_seen = precomputed_block.block_public_keys().into_iter().collect();

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

impl std::fmt::Debug for LedgerDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== LedgerDiff ===")?;
        let mut account_diffs = self.account_diffs.clone();
        account_diffs.sort_by(|x, y| {
            x.public_key()
                .to_address()
                .cmp(&y.public_key().to_address())
        });
        for account_diff in account_diffs.iter() {
            writeln!(f, "{account_diff:?}")?;
        }
        Ok(())
    }
}
