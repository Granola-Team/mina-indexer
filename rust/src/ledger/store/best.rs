//! Store of the best ledger

use crate::{
    block::{store::DbBlockUpdate, BlockHash},
    ledger::{account::Account, diff::account::AccountDiff, public_key::PublicKey, Ledger},
    store::DbUpdate,
};
use speedb::{DBIterator, IteratorMode};
use std::collections::HashSet;

pub trait BestLedgerStore {
    /// Get pk's best ledger account
    fn get_best_account(&self, pk: &PublicKey) -> anyhow::Result<Option<Account>>;

    /// Get the display view of pk's account
    /// ****************************************************************
    /// This is `pk`'s balance accounting for any potential creation fee
    /// ****************************************************************
    fn get_best_account_display(&self, pk: &PublicKey) -> anyhow::Result<Option<Account>>;

    /// Get the best ledger
    fn get_best_ledger(&self, memoize: bool) -> anyhow::Result<Option<Ledger>>;

    /// Update pk's best ledger account
    fn update_best_account(&self, pk: &PublicKey, account: Option<Account>) -> anyhow::Result<()>;

    /// Updates best ledger accounts
    fn update_best_accounts(
        &self,
        state_hash: &BlockHash,
        updates: &DbAccountUpdate,
    ) -> anyhow::Result<()>;

    fn update_block_best_accounts(
        &self,
        state_hash: &BlockHash,
        blocks: &DbBlockUpdate,
    ) -> anyhow::Result<()>;

    /// Remove pk delegation
    fn remove_pk_delegate(&self, pk: PublicKey) -> anyhow::Result<()>;

    /// Add pk delegation
    fn add_pk_delegate(&self, pk: &PublicKey, delegate: &PublicKey) -> anyhow::Result<()>;

    /// Get pk's number of delegations
    fn get_num_pk_delegations(&self, pk: &PublicKey) -> anyhow::Result<u32>;

    /// Get `pk`'s `idx`-th delegation
    fn get_pk_delegation(&self, pk: &PublicKey, idx: u32) -> anyhow::Result<Option<PublicKey>>;

    /// Update best ledger accounts count
    fn update_num_accounts(&self, adjust: i32) -> anyhow::Result<()>;

    /// Get best ledger accounts count
    fn get_num_accounts(&self) -> anyhow::Result<Option<u32>>;

    /// Build the best ledger from the CF representation
    fn build_best_ledger(&self) -> anyhow::Result<Option<Ledger>>;

    ///////////////
    // Iterators //
    ///////////////

    /// Iterator for balance-sorted best ledger accounts
    fn best_ledger_account_balance_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;
}

/// Applied & unapplied block account diffs & new block accounts
pub type DbAccountUpdate = DbUpdate<(Vec<AccountDiff>, HashSet<PublicKey>)>;

impl DbAccountUpdate {
    pub fn new(
        apply: Vec<(Vec<AccountDiff>, HashSet<PublicKey>)>,
        unapply: Vec<(Vec<AccountDiff>, HashSet<PublicKey>)>,
    ) -> Self {
        Self { apply, unapply }
    }
}
