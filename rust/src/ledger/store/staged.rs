//! Store of staged ledgers

use crate::{
    block::BlockHash,
    ledger::{account::Account, diff::LedgerDiff, public_key::PublicKey, Ledger, LedgerHash},
    store::{balance_key_prefix, pk_key_prefix},
};
use speedb::{DBIterator, Direction};
use std::mem::size_of;

pub trait StagedLedgerStore {
    // Get `pk`'s `state_hash` staged ledger account
    fn get_staged_account(
        &self,
        pk: PublicKey,
        state_hash: BlockHash,
    ) -> anyhow::Result<Option<Account>>;

    // Get the display view of `pk`'s `state_hash` staged ledger account
    fn get_staged_account_display(
        &self,
        pk: PublicKey,
        state_hash: BlockHash,
    ) -> anyhow::Result<Option<Account>>;

    // Get `pk`'s `block_height` (canonical) staged ledger account
    fn get_staged_account_block_height(
        &self,
        pk: PublicKey,
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
        pk: PublicKey,
        state_hash: BlockHash,
        account: &Account,
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
    fn set_block_ledger_diff(
        &self,
        state_hash: &BlockHash,
        ledger_diff: LedgerDiff,
    ) -> anyhow::Result<()>;

    /// Index the block's ledger diff
    fn set_block_staged_ledger_hash(
        &self,
        state_hash: &BlockHash,
        staged_ledger_hash: &LedgerHash,
    ) -> anyhow::Result<()>;

    fn get_block_staged_ledger_hash(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<LedgerHash>>;

    /// Get the staged ledger's corresponding block's state hash
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
    state_hash: BlockHash,
    pk: PublicKey,
) -> [u8; BlockHash::LEN + PublicKey::LEN] {
    let mut res = [0u8; BlockHash::LEN + PublicKey::LEN]; // Create a fixed-size array using BlockHash::LEN * 2
    res[..BlockHash::LEN].copy_from_slice(&state_hash.to_bytes()); // Copy the state_hash bytes
    res[BlockHash::LEN..].copy_from_slice(&pk.to_bytes()); // Copy the public key bytes
    res
}

/// Key format for sorting staged ledger accounts by balance
/// ```
/// {state_hash}{balance}{pk}
/// where
/// - state_hash: [BlockHash::LEN] bytes
/// - balance:    8 BE bytes
/// - pk:         [PublicKey::LEN] bytes
pub fn staged_account_balance_sort_key(
    state_hash: BlockHash,
    balance: u64,
    pk: PublicKey,
) -> Vec<u8> {
    let mut res = state_hash.to_bytes().to_vec();
    res.append(&mut balance.to_be_bytes().to_vec());
    res.append(&mut pk.to_bytes());
    res
}

/// Split [staged_account_balance_sort_key] into constituent parts
pub fn split_staged_account_balance_sort_key(key: &[u8]) -> Option<(BlockHash, u64, PublicKey)> {
    if key.len() == BlockHash::LEN + size_of::<u64>() + PublicKey::LEN {
        let state_hash = BlockHash::from_bytes(&key[..BlockHash::LEN]).expect("block hash");
        let balance = balance_key_prefix(&key[BlockHash::LEN..]);
        let pk = pk_key_prefix(&key[BlockHash::LEN + size_of::<u64>()..]);
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
        let mock_state_hash = BlockHash::default(); // Assuming BlockHash::default() provides a valid instance
        let mock_pk = PublicKey::default(); // Assuming PublicKey::default() provides a valid instance

        let result = staged_account_key(mock_state_hash, mock_pk);

        // Assert the length of the result is BlockHash::LEN + PublicKey::LEN
        assert_eq!(result.len(), BlockHash::LEN + PublicKey::LEN);
    }

    #[test]
    fn test_staged_account_key_content() {
        // Mock a BlockHash and PublicKey with specific known values
        let mock_state_hash = BlockHash::default(); // Simulated hash of all 1s
        let mock_pk = PublicKey::default(); // Simulated public key of all 2s

        let result = staged_account_key(mock_state_hash.clone(), mock_pk.clone());

        // Assert the first BlockHash::LEN bytes match the state_hash
        assert_eq!(&result[..BlockHash::LEN], &mock_state_hash.to_bytes()[..]);

        // Assert the remaining bytes match the public key
        assert_eq!(&result[BlockHash::LEN..], &mock_pk.to_bytes()[..]);
    }
}
