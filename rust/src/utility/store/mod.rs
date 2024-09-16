use crate::{block::BlockHash, ledger::public_key::PublicKey};
use anyhow::bail;
use std::mem::size_of;

pub mod block;
pub mod command;
pub mod snarks;
pub mod staking_ledger;

pub(crate) const U32_LEN: usize = size_of::<u32>();
pub(crate) const U64_LEN: usize = size_of::<u64>();
pub(crate) const I64_LEN: usize = size_of::<i64>();

pub fn u32_from_be_bytes(u32_be_bytes: &[u8]) -> anyhow::Result<u32> {
    if u32_be_bytes.len() != U32_LEN {
        bail!("Invalid u32 bytes len: {}", u32_be_bytes.len())
    }

    let mut be_bytes = [0; U32_LEN];
    be_bytes.copy_from_slice(u32_be_bytes);
    Ok(u32::from_be_bytes(be_bytes))
}

pub fn u64_from_be_bytes(u64_be_bytes: &[u8]) -> anyhow::Result<u64> {
    if u64_be_bytes.len() != U64_LEN {
        bail!("Invalid u64 bytes len: {}", u64_be_bytes.len())
    }

    let mut be_bytes = [0; U64_LEN];
    be_bytes.copy_from_slice(u64_be_bytes);
    Ok(u64::from_be_bytes(be_bytes))
}

pub fn i64_from_be_bytes(i64_be_bytes: &[u8]) -> anyhow::Result<i64> {
    if i64_be_bytes.len() != I64_LEN {
        bail!("Invalid i64 bytes len: {}", i64_be_bytes.len())
    }

    let mut be_bytes = [0; I64_LEN];
    be_bytes.copy_from_slice(i64_be_bytes);
    Ok(i64::from_be_bytes(be_bytes))
}

/// Key format
/// ```
/// {pk}{index}
/// where
/// - pk:    [PublicKey] bytes
/// - index: u32 BE bytes
pub fn pk_index_key(pk: &PublicKey, index: u32) -> [u8; PublicKey::LEN + U32_LEN] {
    let mut key = [0; PublicKey::LEN + U32_LEN];
    key[..PublicKey::LEN].copy_from_slice(pk.0.as_bytes());
    key[PublicKey::LEN..].copy_from_slice(&index.to_be_bytes());
    key
}

/// Extracts state hash suffix from the iterator key.
/// Used with [blocks_height_iterator] & [blocks_global_slot_iterator]
pub fn state_hash_suffix(key: &[u8]) -> anyhow::Result<BlockHash> {
    BlockHash::from_bytes(&key[key.len() - BlockHash::LEN..])
}

/// Extracts u32 BE prefix from the iterator key.
/// Used with [blocks_height_iterator] & [blocks_global_slot_iterator]
pub fn block_u32_prefix_from_key(key: &[u8]) -> anyhow::Result<u32> {
    u32_from_be_bytes(&key[..U32_LEN])
}

pub fn from_be_bytes(bytes: Vec<u8>) -> u32 {
    assert_eq!(bytes.len(), U32_LEN);
    let mut be_bytes = [0; U32_LEN];
    be_bytes.copy_from_slice(&bytes);
    u32::from_be_bytes(be_bytes)
}

/// The first [U32_LEN] bytes are `prefix` in big endian
/// - `prefix`: block length, global slot, epoch number, etc
/// - `pk`:     public key
pub fn u32_prefix_key(prefix: u32, pk: &PublicKey) -> [u8; U32_LEN + PublicKey::LEN] {
    let mut bytes = [0; U32_LEN + PublicKey::LEN];
    bytes[..U32_LEN].copy_from_slice(&prefix.to_be_bytes());
    bytes[U32_LEN..].copy_from_slice(pk.0.as_bytes());
    bytes
}

/// The first [U64_LEN] bytes are `prefix` in big endian
/// - `prefix`: balance, fee, etc
/// - `pk`:     [PublicKey] bytes
pub fn u64_prefix_key(prefix: u64, pk: &PublicKey) -> [u8; U64_LEN + PublicKey::LEN] {
    let mut bytes = [0; U64_LEN + PublicKey::LEN];
    bytes[..U64_LEN].copy_from_slice(&prefix.to_be_bytes());
    bytes[U64_LEN..].copy_from_slice(pk.0.as_bytes());
    bytes
}

/// Parse the first [PublicKey::LEN] bytes
pub fn pk_key_prefix(key: &[u8]) -> PublicKey {
    assert!(key.len() >= PublicKey::LEN);
    PublicKey::from_bytes(&key[..PublicKey::LEN]).expect("public key bytes")
}

/// Parse the first [U64_LEN] bytes
pub fn balance_key_prefix(key: &[u8]) -> u64 {
    u64_from_be_bytes(&key[..U64_LEN]).expect("u64 balance BE bytes")
}

/// Drop [PublicKey::LEN] bytes & parse the next [U32_LEN] bytes
pub fn pk_txn_sort_key_sort(key: &[u8]) -> u32 {
    u32_from_be_bytes(&key[PublicKey::LEN..][..U32_LEN]).expect("u32 sort BE bytes")
}

#[cfg(test)]
mod tests {
    use crate::{
        block::BlockHash,
        command::signed::TxnHash,
        ledger::public_key::PublicKey,
        utility::store::{
            command::user::{pk_txn_sort_key, pk_txn_sort_key_prefix, txn_sort_key},
            u32_prefix_key, u64_prefix_key, U32_LEN, U64_LEN,
        },
    };

    #[test]
    fn test_txn_sort_key() {
        let prefix = 99;
        let state_hash = BlockHash::default();
        let txn_hash = TxnHash("a".repeat(TxnHash::LEN));
        let key = txn_sort_key(prefix, &txn_hash, &state_hash);

        assert_eq!(&key[..U32_LEN], &prefix.to_be_bytes());
        assert_eq!(&key[U32_LEN..][..TxnHash::LEN], txn_hash.0.as_bytes());
        assert_eq!(&key[U32_LEN..][TxnHash::LEN..], state_hash.0.as_bytes());
    }

    #[test]
    fn test_pk_txn_sort_key_content() {
        let sort = 500;
        let nonce = 987654321;
        let pk = PublicKey::default();
        let txn_hash = TxnHash("b".repeat(TxnHash::LEN));
        let state_hash = BlockHash::default();
        let key = pk_txn_sort_key(&pk, sort, nonce, &txn_hash, &state_hash);

        assert_eq!(&key[..PublicKey::LEN], pk.0.as_bytes());
        assert_eq!(&key[PublicKey::LEN..][..U32_LEN], &sort.to_be_bytes());
        assert_eq!(
            &key[PublicKey::LEN..][U32_LEN..][..U32_LEN],
            &nonce.to_be_bytes()
        );
        assert_eq!(
            &key[PublicKey::LEN..][U32_LEN..][U32_LEN..][..TxnHash::LEN],
            txn_hash.0.as_bytes()
        );
        assert_eq!(
            &key[PublicKey::LEN..][U32_LEN..][U32_LEN..][TxnHash::LEN..],
            state_hash.0.as_bytes()
        );
    }

    #[test]
    fn test_pk_txn_sort_key_prefix() {
        let sort = 12345;
        let pk = PublicKey::default();
        let key = pk_txn_sort_key_prefix(&pk, sort);

        assert_eq!(&key[..PublicKey::LEN], pk.0.as_bytes());
        assert_eq!(&key[PublicKey::LEN..], &sort.to_be_bytes());
    }

    #[test]
    fn test_u32_prefix_key_with_valid_inputs() {
        let prefix = 42;
        let public_key = PublicKey::default();
        let key = u32_prefix_key(prefix, &public_key);

        assert_eq!(&key[..U32_LEN], &prefix.to_be_bytes());
        assert_eq!(&key[U32_LEN..], public_key.0.as_bytes());
    }

    #[test]
    fn test_u64_prefix_key() {
        // Test case 1: Check if the prefix and suffix are correctly combined
        let prefix = 1234567890;
        let pk = PublicKey::default();
        let key = u64_prefix_key(prefix, &pk);

        assert_eq!(&key[..U64_LEN], &prefix.to_be_bytes());
        assert_eq!(&key[U64_LEN..], pk.0.as_bytes());
    }

    #[test]
    fn test_u64_prefix_key_with_different_values() {
        // Test case 2: Use a different prefix and suffix and ensure correctness
        let prefix = u64::MAX;
        let pk = PublicKey::default();
        let key = u64_prefix_key(prefix, &pk);

        assert_eq!(&key[..U64_LEN], &prefix.to_be_bytes());
        assert_eq!(&key[U64_LEN..], pk.0.as_bytes());
    }
}
