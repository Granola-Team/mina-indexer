//! Store of staged ledgers

use crate::{
    block::BlockHash,
    ledger::{
        account::Account, diff::LedgerDiff, public_key::PublicKey, token::TokenAddress, Ledger,
        LedgerHash,
    },
};
use speedb::{DBIterator, Direction, WriteBatch};

pub trait StagedLedgerStore {
    // Get `pk`'s `state_hash` staged ledger account
    fn get_staged_account(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<Account>>;

    // Get the display view of `pk`'s `state_hash` staged ledger account
    /// ****************************************************************
    /// This is `pk`'s balance accounting for any potential creation fee
    /// ****************************************************************
    fn get_staged_account_display(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<Account>>;

    // Get `pk`'s `block_height` (canonical) staged ledger account
    fn get_staged_account_block_height(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        block_height: u32,
    ) -> anyhow::Result<Option<Account>>;

    /// Get a ledger associated with ledger hash
    fn get_staged_ledger_at_ledger_hash(
        &self,
        ledger_hash: &LedgerHash,
        memoize: bool,
    ) -> anyhow::Result<Option<Ledger>>;

    /// Get a ledger associated with an arbitrary block
    fn get_staged_ledger_at_state_hash(
        &self,
        state_hash: &BlockHash,
        memoize: bool,
    ) -> anyhow::Result<Option<Ledger>>;

    /// Get a (canonical) ledger at a specified block height
    /// (i.e. blockchain_length)
    fn get_staged_ledger_at_block_height(
        &self,
        height: u32,
        memoize: bool,
    ) -> anyhow::Result<Option<Ledger>>;

    /// Set `pk`'s `state_hash` staged ledger `account` & balance-sort data
    fn set_staged_account(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        state_hash: &BlockHash,
        block_height: u32,
        account: &Account,
    ) -> anyhow::Result<()>;

    // Set a staged ledger account with the raw serde bytes
    fn set_staged_account_raw_bytes(
        &self,
        pk: &PublicKey,
        token: &TokenAddress,
        state_hash: &BlockHash,
        balance: u64,
        block_height: u32,
        account_serde_bytes: &[u8],
    ) -> anyhow::Result<()>;

    // Get pk's minimum staged ledger account block height
    fn get_pk_min_staged_ledger_block(&self, pk: &PublicKey) -> anyhow::Result<Option<u32>>;

    // Set pk's minimum staged ledger account block height
    fn set_pk_min_staged_ledger_block(
        &self,
        pk: &PublicKey,
        block_height: u32,
    ) -> anyhow::Result<()>;

    /// Add a ledger with assoociated hashes
    /// Returns true if ledger already present
    fn add_staged_ledger_hashes(
        &self,
        ledger_hash: &LedgerHash,
        state_hash: &BlockHash,
    ) -> anyhow::Result<bool>;

    /// Add a ledger associated with a canonical block
    fn add_staged_ledger_at_state_hash(
        &self,
        state_hash: &BlockHash,
        ledger: Ledger,
        block_height: u32,
    ) -> anyhow::Result<()>;

    /// Add a new genesis ledger
    fn add_genesis_ledger(
        &self,
        state_hash: &BlockHash,
        genesis_ledger: Ledger,
        block_height: u32,
    ) -> anyhow::Result<()>;

    /// Index the block's ledger diff
    fn set_block_ledger_diff_batch(
        &self,
        state_hash: &BlockHash,
        ledger_diff: &LedgerDiff,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()>;

    /// Index the block's ledger diff
    fn set_block_staged_ledger_hash_batch(
        &self,
        state_hash: &BlockHash,
        staged_ledger_hash: &LedgerHash,
        batch: &mut WriteBatch,
    ) -> anyhow::Result<()>;

    /// Get the block's corresponding staged ledger hash
    fn get_block_staged_ledger_hash(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<LedgerHash>>;

    /// Get the staged ledger's corresponding block state hash
    fn get_staged_ledger_block_state_hash(
        &self,
        ledger_hash: &LedgerHash,
    ) -> anyhow::Result<Option<BlockHash>>;

    /// Build the `state_hash` staged ledger from the CF representation
    fn build_staged_ledger(&self, state_hash: &BlockHash) -> anyhow::Result<Option<Ledger>>;

    ///////////////
    // Iterators //
    ///////////////

    /// Iterator for balance-sorted staged ledger accounts
    /// (key: [staged_account_balance_sort_key])
    fn staged_ledger_account_balance_iterator(
        &self,
        state_hash: &BlockHash,
        direction: Direction,
    ) -> DBIterator<'_>;
}
