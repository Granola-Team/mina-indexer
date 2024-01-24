pub mod account;

use crate::{
    block::precomputed::PrecomputedBlock,
    ledger::{coinbase::Coinbase, diff::account::AccountDiff, PublicKey},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct LedgerDiff {
    pub public_keys_seen: Vec<PublicKey>,
    pub account_diffs: Vec<AccountDiff>,
}

impl LedgerDiff {
    /// Compute a ledger diff from the given precomputed block
    pub fn from_precomputed(precomputed_block: &PrecomputedBlock) -> Self {
        let coinbase = Coinbase::from_precomputed(precomputed_block);
        let mut account_diff_fees: Vec<AccountDiff> =
            AccountDiff::from_block_fees(coinbase.receiver.clone(), precomputed_block);
        let mut account_diff_transactions = precomputed_block
            .commands()
            .into_iter()
            .filter(|x| x.is_applied())
            .map(|x| x.to_command())
            .flat_map(AccountDiff::from_command)
            .collect();

        let mut account_diffs = Vec::new();
        account_diffs.append(&mut account_diff_fees);
        account_diffs.append(&mut account_diff_transactions);

        // apply coinbase last
        if let Some(coinbase_update) = coinbase.as_account_diff() {
            account_diffs.push(coinbase_update);
        }

        LedgerDiff {
            public_keys_seen: precomputed_block.active_public_keys(),
            account_diffs,
        }
    }

    pub fn append(&mut self, other: Self) {
        other.public_keys_seen.into_iter().for_each(|account| {
            if !self.public_keys_seen.contains(&account) {
                self.public_keys_seen.push(account);
            }
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
