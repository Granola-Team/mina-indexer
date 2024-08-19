//! Store of the best ledger

use crate::{
    block::BlockHash,
    ledger::{
        account::Account,
        diff::account::{AccountDiff, UpdateType},
        public_key::PublicKey,
        Ledger,
    },
    store::DBUpdate,
};
use speedb::{DBIterator, IteratorMode};
use std::collections::HashMap;

pub trait BestLedgerStore {
    /// Get the best ledger (associated with the best block)
    fn get_best_ledger(&self) -> anyhow::Result<Option<Ledger>>;

    /// Update pk's account
    fn update_best_account(&self, pk: &PublicKey, account: Option<Account>) -> anyhow::Result<()>;

    /// Updates balance-sorted accounts
    fn update_best_accounts(
        &self,
        state_hash: &BlockHash,
        updates: &DBAccountUpdate,
    ) -> anyhow::Result<()>;

    /// Generate account balance updates when the best tip changes.
    /// Return with set of coinbase receivers.
    fn reorg_account_updates(
        &self,
        old_best_tip: &BlockHash,
        new_best_tip: &BlockHash,
    ) -> anyhow::Result<DBAccountUpdate>;

    /// Get pk's account balance
    fn get_best_account(&self, pk: &PublicKey) -> anyhow::Result<Option<Account>>;

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

    ///////////////
    // Iterators //
    ///////////////

    /// Iterator for balance-sorted accounts
    /// `{balance}{pk} -> _`
    /// ```
    /// - balance: 8 BE bytes
    /// - pk:      [PublicKey::LEN] bytes
    fn account_balance_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;
}

pub type DBAccountUpdate = DBUpdate<AccountDiff>;

impl DBAccountUpdate {
    pub fn new(apply: Vec<AccountDiff>, unapply: Vec<AccountDiff>) -> Self {
        Self { apply, unapply }
    }
}
