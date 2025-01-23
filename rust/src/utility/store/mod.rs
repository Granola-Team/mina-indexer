pub mod block;
pub mod command;
pub mod common;
pub mod ledger;
pub mod snarks;
pub mod zkapp;

#[cfg(test)]
mod tests {
    use crate::{
        block::BlockHash,
        command::signed::TxnHash,
        ledger::public_key::PublicKey,
        utility::store::{
            command::user::{pk_txn_sort_key, pk_txn_sort_key_prefix, txn_sort_key},
            common::{u32_prefix_key, u64_prefix_key, U32_LEN, U64_LEN},
        },
    };

    #[test]
    fn test_txn_sort_key() {
        let prefix = 99;
        let state_hash = BlockHash::default();
        let txn_hash = TxnHash::V1("a".repeat(TxnHash::V1_LEN));
        let key = txn_sort_key(prefix, &txn_hash, &state_hash);

        assert_eq!(&key[..U32_LEN], &prefix.to_be_bytes());
        assert_eq!(
            &key[U32_LEN..][..TxnHash::V1_LEN],
            txn_hash.to_string().as_bytes()
        );
        assert_eq!(&key[U32_LEN..][TxnHash::V1_LEN..], state_hash.0.as_bytes());
    }

    #[test]
    fn test_pk_txn_sort_key_content() {
        let sort = 500;
        let nonce = 987654321;
        let pk = PublicKey::default();
        let txn_hash = TxnHash::V1("b".repeat(TxnHash::V1_LEN));
        let state_hash = BlockHash::default();
        let key = pk_txn_sort_key(&pk, sort, nonce, &txn_hash, &state_hash);

        assert_eq!(&key[..PublicKey::LEN], pk.0.as_bytes());
        assert_eq!(&key[PublicKey::LEN..][..U32_LEN], &sort.to_be_bytes());
        assert_eq!(
            &key[PublicKey::LEN..][U32_LEN..][..U32_LEN],
            &nonce.to_be_bytes()
        );
        assert_eq!(
            &key[PublicKey::LEN..][U32_LEN..][U32_LEN..][..TxnHash::V1_LEN],
            txn_hash.to_string().as_bytes()
        );
        assert_eq!(
            &key[PublicKey::LEN..][U32_LEN..][U32_LEN..][TxnHash::V1_LEN..],
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
