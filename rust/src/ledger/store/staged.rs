//! Staged ledger store trait

use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    ledger::{
        account::Account,
        diff::LedgerDiff,
        token::{Token, TokenAddress},
        Ledger, LedgerHash,
    },
    store::Result,
};
use speedb::{DBIterator, Direction, WriteBatch};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct StateHashWithHeight {
    pub state_hash: StateHash,
    pub blockchain_length: u32,
}

pub trait StagedLedgerStore {
    // Get `pk`'s `state_hash` staged ledger account
    fn get_staged_account(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        state_hash: &StateHash,
    ) -> Result<Option<Account>>;

    // Get the display view of `pk`'s `state_hash` staged ledger account
    /// ****************************************************************
    /// This is `pk`'s balance accounting for any potential creation fee
    /// ****************************************************************
    fn get_staged_account_display(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        state_hash: &StateHash,
    ) -> Result<Option<Account>>;

    // Get `pk`'s `block_height` (canonical) staged ledger account
    fn get_staged_account_block_height(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        block_height: u32,
    ) -> Result<Option<Account>>;

    /// Get a ledger associated with ledger hash
    fn get_staged_ledger_at_ledger_hash(
        &self,
        ledger_hash: &LedgerHash,
        memoize: bool,
    ) -> Result<Option<Ledger>>;

    /// Get a ledger associated with an arbitrary block
    fn get_staged_ledger_at_state_hash(
        &self,
        state_hash: &StateHash,
        memoize: bool,
    ) -> Result<Option<Ledger>>;

    /// Get a (canonical) ledger at a specified block height
    /// (i.e. blockchain_length)
    fn get_staged_ledger_at_block_height(
        &self,
        height: u32,
        memoize: bool,
    ) -> Result<Option<Ledger>>;

    /// Set `pk`'s `state_hash` staged ledger `account` & balance-sort data
    fn set_staged_account(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        state_hash: &StateHash,
        block_height: u32,
        account: &Account,
    ) -> Result<()>;

    // Set a staged ledger account with the raw serde bytes
    fn set_staged_account_raw_bytes(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        state_hash: &StateHash,
        balance: u64,
        block_height: u32,
        account_serde_bytes: &[u8],
    ) -> Result<()>;

    /// Remove a staged ledger account when a block is unapplied
    fn remove_staged_account(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        state_hash: &StateHash,
        block_height: u32,
        balance: u64,
    ) -> Result<()>;

    // Get pk's minimum staged ledger account state hash & block height
    fn get_pk_min_staged_ledger_block(&self, pk: &PublicKey)
        -> Result<Option<StateHashWithHeight>>;

    // Set pk's minimum staged ledger account state hash & block height
    fn set_pk_min_staged_ledger_block(
        &self,
        pk: &PublicKey,
        block_info: &StateHashWithHeight,
    ) -> Result<()>;

    /// Add a staged ledger hash with associated the given state hash
    ///
    /// Returns true if ledger already present
    fn add_staged_ledger_hash(
        &self,
        ledger_hash: &LedgerHash,
        state_hash: &StateHash,
    ) -> Result<bool>;

    /// Add a ledger associated with a canonical block
    fn add_staged_ledger_at_state_hash(
        &self,
        state_hash: &StateHash,
        ledger: &Ledger,
        block_height: u32,
    ) -> Result<()>;

    /// Add a new genesis ledger
    fn add_genesis_ledger(
        &self,
        state_hash: &StateHash,
        genesis_ledger: &Ledger,
        block_height: u32,
        genesis_token: Option<&Token>,
    ) -> Result<()>;

    /// Index the block's ledger diff
    fn set_block_ledger_diff_batch(
        &self,
        state_hash: &StateHash,
        ledger_diff: &LedgerDiff,
        batch: &mut WriteBatch,
    ) -> Result<()>;

    /// Index the block's ledger diff
    fn set_block_staged_ledger_hash_batch(
        &self,
        state_hash: &StateHash,
        staged_ledger_hash: &LedgerHash,
        batch: &mut WriteBatch,
    ) -> Result<()>;

    /// Get the block's corresponding staged ledger hash
    fn get_block_staged_ledger_hash(&self, state_hash: &StateHash) -> Result<Option<LedgerHash>>;

    /// Get the staged ledger's corresponding block state hash
    fn get_staged_ledger_block_state_hash(
        &self,
        ledger_hash: &LedgerHash,
    ) -> Result<Option<StateHash>>;

    /// Build the `state_hash` staged ledger from the CF representation
    fn build_staged_ledger(&self, state_hash: &StateHash) -> Result<Option<Ledger>>;

    ///////////////
    // Iterators //
    ///////////////

    /// Iterator for balance-sorted staged ledger accounts
    /// (key: [staged_account_balance_sort_key])
    fn staged_ledger_account_balance_iterator(
        &self,
        state_hash: &StateHash,
        direction: Direction,
    ) -> DBIterator<'_>;
}
