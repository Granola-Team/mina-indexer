use crate::{
    block::BlockHash,
    ledger::{diff::account::PaymentDiff, public_key::PublicKey},
};
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
}
