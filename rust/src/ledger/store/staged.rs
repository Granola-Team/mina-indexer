//! Store of staged ledgers

use crate::{
    block::BlockHash,
    ledger::{account::Account, diff::LedgerDiff, public_key::PublicKey, Ledger, LedgerHash},
    utility::store::{balance_key_prefix, pk_key_prefix, U64_LEN},
};
use speedb::{DBIterator, Direction, WriteBatch};

pub trait StagedLedgerStore {
    // Get `pk`'s `state_hash` staged ledger account
    fn get_staged_account(
        &self,
        pk: &PublicKey,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<Account>>;

    // Get the display view of `pk`'s `state_hash` staged ledger account
    /// ****************************************************************
    /// This is `pk`'s balance accounting for any potential creation fee
    /// ****************************************************************
    fn get_staged_account_display(
        &self,
        pk: &PublicKey,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<Account>>;

    // Get `pk`'s `block_height` (canonical) staged ledger account
    fn get_staged_account_block_height(
        &self,
        pk: &PublicKey,
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
        state_hash: &BlockHash,
        account: &Account,
    ) -> anyhow::Result<()>;

    // Set a staged ledger account with the raw serde bytes
    fn set_staged_account_raw_bytes(
        &self,
        pk: &PublicKey,
        state_hash: &BlockHash,
        balance: u64,
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
    ) -> anyhow::Result<()>;

    /// Add a new genesis ledger
    fn add_genesis_ledger(
        &self,
        state_hash: &BlockHash,
        genesis_ledger: Ledger,
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

/// Key format for storing staged ledger accounts by state hash
/// ```
/// {state_hash}{pk}
/// where
/// - state_hash:   [BlockHash::LEN] bytes
/// - pk:           [PublicKey::LEN] bytes
pub fn staged_account_key(
    state_hash: &BlockHash,
    pk: &PublicKey,
) -> [u8; BlockHash::LEN + PublicKey::LEN] {
    let mut key = [0; BlockHash::LEN + PublicKey::LEN];
    key[..BlockHash::LEN].copy_from_slice(state_hash.0.as_bytes());
    key[BlockHash::LEN..].copy_from_slice(pk.0.as_bytes());
    key
}

/// Key format for sorting staged ledger accounts by balance
/// ```
/// {state_hash}{balance}{pk}
/// where
/// - state_hash: [BlockHash::LEN] bytes
/// - balance:    8 BE bytes
/// - pk:         [PublicKey::LEN] bytes
pub fn staged_account_balance_sort_key(
    state_hash: &BlockHash,
    balance: u64,
    pk: &PublicKey,
) -> [u8; BlockHash::LEN + U64_LEN + PublicKey::LEN] {
    let mut res = [0; BlockHash::LEN + U64_LEN + PublicKey::LEN];
    res[..BlockHash::LEN].copy_from_slice(state_hash.0.as_bytes());
    res[BlockHash::LEN..][..U64_LEN].copy_from_slice(&balance.to_be_bytes());
    res[BlockHash::LEN..][U64_LEN..].copy_from_slice(pk.0.as_bytes());
    res
}

/// Split [staged_account_balance_sort_key] into constituent parts
pub fn split_staged_account_balance_sort_key(key: &[u8]) -> Option<(BlockHash, u64, PublicKey)> {
    if key.len() == BlockHash::LEN + U64_LEN + PublicKey::LEN {
        let state_hash = BlockHash::from_bytes(&key[..BlockHash::LEN]).expect("block hash");
        let balance = balance_key_prefix(&key[BlockHash::LEN..]);
        let pk = pk_key_prefix(&key[BlockHash::LEN + U64_LEN..]);
        return Some((state_hash, balance, pk));
    }
    None
}

#[cfg(test)]
mod staged_tests {
    use super::*;
    use crate::{block::BlockHash, ledger::public_key::PublicKey};

    #[test]
    fn test_staged_account_key_length() {
        // Mock a BlockHash and PublicKey with known sizes
        let mock_state_hash = BlockHash::default();
        let mock_pk = PublicKey::default();

        // Assert the length of the result is BlockHash::LEN + PublicKey::LEN
        assert_eq!(
            staged_account_key(&mock_state_hash, &mock_pk).len(),
            BlockHash::LEN + PublicKey::LEN
        );
    }

    #[test]
    fn test_staged_account_key_content() {
        // Mock a BlockHash and PublicKey with specific known values
        let mock_state_hash = BlockHash::default();
        let mock_pk = PublicKey::default();
        let result = staged_account_key(&mock_state_hash, &mock_pk);

        // Assert the first BlockHash::LEN bytes match the state_hash
        assert_eq!(&result[..BlockHash::LEN], mock_state_hash.0.as_bytes());

        // Assert the remaining bytes match the public key
        assert_eq!(&result[BlockHash::LEN..], mock_pk.0.as_bytes());
    }

    #[test]
    fn test_staged_account_balance_sort_key_length() -> anyhow::Result<()> {
        // Mock inputs
        let state_hash = BlockHash::default();
        let balance = 123456789u64;
        let pk = PublicKey::default();

        // Check that the result has the correct length
        assert_eq!(
            staged_account_balance_sort_key(&state_hash, balance, &pk).len(),
            BlockHash::LEN + U64_LEN + PublicKey::LEN
        );

        Ok(())
    }

    #[test]
    fn test_staged_account_balance_sort_key_content() -> anyhow::Result<()> {
        // Mock inputs
        let state_hash = BlockHash::default(); // Use default for BlockHash
        let balance = 987654321u64; // Mock balance
        let pk = PublicKey::default(); // Use default for PublicKey

        // Generate key
        let result = staged_account_balance_sort_key(&state_hash, balance, &pk);

        // Check the state hash bytes
        assert_eq!(&result[..BlockHash::LEN], state_hash.0.as_bytes());

        // Check the balance bytes (u64, big-endian)
        assert_eq!(&result[BlockHash::LEN..][..U64_LEN], &balance.to_be_bytes());

        // Check the public key bytes
        assert_eq!(&result[BlockHash::LEN..][U64_LEN..], pk.0.as_bytes());

        Ok(())
    }
}
