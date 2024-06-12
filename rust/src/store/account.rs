use crate::{
    block::BlockHash,
    ledger::{diff::account::PaymentDiff, public_key::PublicKey},
};
use speedb::{DBIterator, IteratorMode};
use std::collections::HashSet;

pub trait AccountStore {
    /// Update pk's balance-sorted account balance
    fn update_account_balance(&self, pk: &PublicKey, balance: Option<u64>) -> anyhow::Result<()>;

    /// Generate account balance updates when the best tip changes.
    /// Return with set of coinbase receivers.
    fn common_ancestor_account_balance_updates(
        &self,
        old_best_tip: &BlockHash,
        new_best_tip: &BlockHash,
    ) -> anyhow::Result<(Vec<PaymentDiff>, HashSet<PublicKey>)>;

    /// Set the balance updates for a block
    fn set_block_balance_updates(
        &self,
        state_hash: &BlockHash,
        coinbase_receiver: PublicKey,
        balance_updates: Vec<PaymentDiff>,
    ) -> anyhow::Result<()>;

    /// Get a block's balance updates
    fn get_block_balance_updates(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<(PublicKey, Vec<PaymentDiff>)>>;

    /// Updates stored account balances
    fn update_account_balances(
        &self,
        state_hash: &BlockHash,
        updates: Vec<PaymentDiff>,
        coinbase_receivers: HashSet<PublicKey>,
    ) -> anyhow::Result<()>;

    /// Get pk's account balance
    fn get_account_balance(&self, pk: &PublicKey) -> anyhow::Result<Option<u64>>;

    ///////////////
    // Iterators //
    ///////////////

    /// Iterator for balance-sorted accounts
    /// `{balance}{pk} -> _`
    /// ```
    /// - balance: 8 BE bytes
    /// - pk:      [PublicKey::LEN] bytes
    fn account_balance_iterator<'a>(&'a self, mode: IteratorMode) -> DBIterator<'a>;
}
