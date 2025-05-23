//! Best ledger store trait

use super::update::DbAccountUpdate;
use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::store::DbBlockUpdate,
    ledger::{account::Account, diff::token::TokenDiff, token::TokenAddress, Ledger},
    store::Result,
};
use speedb::{DBIterator, IteratorMode};

pub trait BestLedgerStore {
    /////////////////////////
    // Best ledger account //
    /////////////////////////

    /// Get the token account from the best ledger
    /// ****************************************************
    /// This does not account for any potential creation fee
    /// ****************************************************
    fn get_best_account(&self, pk: &PublicKey, token: &TokenAddress) -> Result<Option<Account>>;

    /// Get the display view of the token account
    /// ********************************************
    /// This accounts for any potential creation fee
    /// ********************************************
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
        is_new_account: bool,
    ) -> Result<()>;

    /// Update the best ledger token accounts
    fn update_best_accounts(
        &self,
        state_hash: &StateHash,
        block_height: u32,
        updates: DbAccountUpdate,
    ) -> Result<()>;

    /// Update the best ledger tokens
    fn apply_best_token_diffs(
        &self,
        state_hash: &StateHash,
        token_diffs: &[TokenDiff],
    ) -> Result<()>;

    /// Update the best ledger token
    fn unapply_best_token_diffs(&self, token_diffs: &[TokenDiff]) -> Result<()>;

    /// Update the best ledger token accounts & tokens due to a block update
    fn update_block_best_accounts(
        &self,
        state_hash: &StateHash,
        block_height: u32,
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

    //////////////////
    // All accounts //
    //////////////////

    /// Update the count of best ledger accounts
    fn update_num_accounts(&self, adjust: i32) -> Result<()>;

    /// Get the count of best ledger accounts
    fn get_num_accounts(&self) -> Result<Option<u32>>;

    /// Set the count of best ledger accounts
    fn set_num_accounts(&self, num: u32) -> Result<()>;

    /// Increment the count of all best ledger accounts
    fn increment_num_accounts(&self) -> Result<()>;

    /// Decrement the count of all best ledger accounts
    fn decrement_num_accounts(&self) -> Result<()>;

    ///////////////////
    // MINA accounts //
    ///////////////////

    /// Update the count of best ledger MINA accounts
    fn update_num_mina_accounts(&self, adjust: i32) -> Result<()>;

    /// Get the count of best ledger MINA accounts
    fn get_num_mina_accounts(&self) -> Result<Option<u32>>;

    /// Set the count of best ledger MINA accounts
    fn set_num_mina_accounts(&self, num: u32) -> Result<()>;

    /// Increment the count of best ledger MINA accounts
    fn increment_num_mina_accounts(&self) -> Result<()>;

    /// Decrement the count of best ledger MINA accounts
    fn decrement_num_mina_accounts(&self) -> Result<()>;

    /// Update the count of best ledger MINA zkapp accounts
    fn update_num_mina_zkapp_accounts(&self, adjust: i32) -> Result<()>;

    /// Get the count of best ledger MINA zkapp accounts
    fn get_num_mina_zkapp_accounts(&self) -> Result<Option<u32>>;

    /// Set the count of best ledger MINA zkapp accounts
    fn set_num_mina_zkapp_accounts(&self, num: u32) -> Result<()>;

    /// Increment the count of best ledger MINA zkapp accounts
    fn increment_num_mina_zkapp_accounts(&self) -> Result<()>;

    /// Decrement the count of best ledger MINA zkapp accounts
    fn decrement_num_mina_zkapp_accounts(&self) -> Result<()>;

    ////////////////////
    // zkApp accounts //
    ////////////////////

    /// Update the count of zkapp best ledger accounts
    fn update_num_zkapp_accounts(&self, adjust: i32) -> Result<()>;

    /// Get the count of best ledger zkapp accounts
    fn get_num_zkapp_accounts(&self) -> Result<Option<u32>>;

    /// Set the count of best ledger zkapp accounts
    fn set_num_zkapp_accounts(&self, num: u32) -> Result<()>;

    /// Increment the count of best ledger zkapp accounts
    fn increment_num_zkapp_accounts(&self) -> Result<()>;

    /// Decrement the count of best ledger zkapp accounts
    fn decrement_num_zkapp_accounts(&self) -> Result<()>;

    /// Check whether a a token account is a zkapp account
    fn is_zkapp_account(&self, pk: &PublicKey, token: &TokenAddress) -> Result<Option<bool>>;

    /////////////////////////
    // Best ledger builder //
    /////////////////////////

    /// Build the best ledger from the CF representation
    fn build_best_ledger(&self) -> Result<Option<Ledger>>;

    /// Get the best ledger
    fn get_best_ledger(&self, memoize: bool) -> Result<Option<Ledger>>;

    ///////////////
    // Iterators //
    ///////////////

    /// Iterator for balance-sorted best ledger accounts
    /// ```
    /// key: {token}{balance}{pk}
    /// val: [Account] serde bytes
    /// where
    /// - token:   [TokenAddress] bytes
    /// - balance: [u64] BE bytes
    /// - pk:      [PublicKey] bytes
    fn best_ledger_account_balance_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;

    /// Iterator for balance-sorted best ledger zkapp accounts
    /// ```
    /// key: {token}{balance}{pk}
    /// val: [Account] serde bytes
    /// where
    /// - token:   [TokenAddress] bytes
    /// - balance: [u64] BE bytes
    /// - pk:      [PublicKey] bytes
    fn zkapp_best_ledger_account_balance_iterator(&self, mode: IteratorMode) -> DBIterator<'_>;
}
