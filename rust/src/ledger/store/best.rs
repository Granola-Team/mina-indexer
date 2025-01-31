//! Store of the best ledger

use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::store::DbBlockUpdate,
    ledger::{account::Account, diff::account::AccountDiff, token::TokenAddress, Ledger},
    store::{DbUpdate, Result},
};
use speedb::{DBIterator, IteratorMode};
use std::collections::HashSet;

pub trait BestLedgerStore {
    /// Get the token account from the best ledger
    fn get_best_account(&self, pk: &PublicKey, token: &TokenAddress) -> Result<Option<Account>>;

    /// Get the display view of the token account
    /// ****************************************************************
    /// This is `pk`'s balance accounting for any potential creation fee
    /// ****************************************************************
    fn get_best_account_display(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
    ) -> Result<Option<Account>>;

    /// Update the best ledger token account
    fn update_best_account(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        before: Option<(bool, u64)>,
        after: Option<Account>,
    ) -> Result<()>;

    /// Update the best ledger token accounts
    fn update_best_accounts(&self, state_hash: &StateHash, updates: DbAccountUpdate) -> Result<()>;

    /// Update the best ledger token accounts due to block updates
    fn update_block_best_accounts(
        &self,
        state_hash: &StateHash,
        blocks: &DbBlockUpdate,
    ) -> Result<()>;

    /// Remove a delegation
    fn remove_pk_delegate(&self, pk: PublicKey) -> Result<()>;

    /// Add a delegation
    fn add_pk_delegate(&self, pk: &PublicKey, delegate: &PublicKey) -> Result<()>;

    /// Get the number of delegations
    fn get_num_pk_delegations(&self, pk: &PublicKey) -> Result<u32>;

    /// Get the `idx`-th delegation
    fn get_pk_delegation(&self, pk: &PublicKey, idx: u32) -> Result<Option<PublicKey>>;

    /// Update the count of best ledger accounts
    fn update_num_accounts(&self, adjust: i32) -> Result<()>;

    /// Get the count of best ledger accounts
    fn get_num_accounts(&self) -> Result<Option<u32>>;

    /// Build the best ledger from the CF representation
    fn build_best_ledger(&self) -> Result<Option<Ledger>>;

    /// Get the best ledger
    fn get_best_ledger(&self, memoize: bool) -> Result<Option<Ledger>>;

    ///////////////
    // Iterators //
    ///////////////

    /// Iterator for balance-sorted best ledger accounts
    /// ```
    /// {token}{balance}{pk} -> _
    /// where
    /// - token:   [TokenAddress] bytes
    /// - balance: [u64] BE bytes
    /// - pk:      [PublicKey] bytes
    fn best_ledger_account_balance_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Iterator for balance-sorted best ledger zkapp accounts
    /// ```
    /// {token}{balance}{pk} -> _
    /// where
    /// - token:   [TokenAddress] bytes
    /// - balance: [u64] BE bytes
    /// - pk:      [PublicKey] bytes
    fn zkapp_best_ledger_account_balance_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;
}

/// Applied & unapplied block account diffs & new block accounts
type AccountUpdate = (Vec<AccountDiff>, HashSet<(PublicKey, TokenAddress)>);
pub type DbAccountUpdate = DbUpdate<AccountUpdate>;

impl DbAccountUpdate {
    pub fn new(apply: Vec<AccountUpdate>, unapply: Vec<AccountUpdate>) -> Self {
        Self { apply, unapply }
    }
}
