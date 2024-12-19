use crate::{
    block::BlockHash,
    ledger::{public_key::PublicKey, token::TokenAddress},
    utility::store::{balance_key_prefix, pk_key_prefix, U64_LEN},
};

/// Key format for storing staged ledger accounts by state hash
/// ```
/// {state_hash}{pk}
/// where
/// - state_hash: [BlockHash::LEN] bytes
/// - token:      [TokenAddress::LEN] bytes
/// - pk:         [PublicKey::LEN] bytes
pub fn staged_account_key(
    state_hash: &BlockHash,
    token: &TokenAddress,
    pk: &PublicKey,
) -> [u8; BlockHash::LEN + TokenAddress::LEN + PublicKey::LEN] {
    let mut key = [0; BlockHash::LEN + TokenAddress::LEN + PublicKey::LEN];

    key[..BlockHash::LEN].copy_from_slice(state_hash.0.as_bytes());
    key[BlockHash::LEN..][..TokenAddress::LEN].copy_from_slice(token.0.as_bytes());
    key[BlockHash::LEN..][TokenAddress::LEN..].copy_from_slice(pk.0.as_bytes());
    key
}

/// Key format for sorting staged ledger accounts by balance
/// ```
/// {state_hash}{token}{balance}{pk}
/// where
/// - state_hash: [BlockHash::LEN] bytes
/// - token:      [TokenAddress::LEN] bytes
/// - balance:    [u64] BE bytes
/// - pk:         [PublicKey::LEN] bytes
pub fn staged_account_balance_sort_key(
    state_hash: &BlockHash,
    token: &TokenAddress,
    balance: u64,
    pk: &PublicKey,
) -> [u8; BlockHash::LEN + TokenAddress::LEN + U64_LEN + PublicKey::LEN] {
    let mut key = [0; BlockHash::LEN + TokenAddress::LEN + U64_LEN + PublicKey::LEN];

    key[..BlockHash::LEN].copy_from_slice(state_hash.0.as_bytes());
    key[BlockHash::LEN..][..TokenAddress::LEN].copy_from_slice(token.0.as_bytes());
    key[BlockHash::LEN..][TokenAddress::LEN..][..U64_LEN].copy_from_slice(&balance.to_be_bytes());
    key[BlockHash::LEN..][TokenAddress::LEN..][U64_LEN..].copy_from_slice(pk.0.as_bytes());
    key
}

/// Split [staged_account_balance_sort_key] into constituent parts
pub fn split_staged_account_balance_sort_key(
    key: &[u8],
) -> Option<(BlockHash, TokenAddress, u64, PublicKey)> {
    if key.len() == BlockHash::LEN + TokenAddress::LEN + U64_LEN + PublicKey::LEN {
        let state_hash = BlockHash::from_bytes(&key[..BlockHash::LEN]).expect("block hash");
        let token = TokenAddress::from_bytes(key[BlockHash::LEN..][..TokenAddress::LEN].to_vec())
            .expect("token address");

        let balance = balance_key_prefix(&key[BlockHash::LEN..][TokenAddress::LEN..]);
        let pk = pk_key_prefix(&key[BlockHash::LEN..][TokenAddress::LEN..][U64_LEN..]);

        return Some((state_hash, token, balance, pk));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{block::BlockHash, ledger::public_key::PublicKey};

    #[test]
    fn test_staged_account_key_length() {
        let state_hash = BlockHash::default();
        let token = TokenAddress::default();
        let pk = PublicKey::default();

        // key has the expected length
        assert_eq!(
            staged_account_key(&state_hash, &token, &pk).len(),
            BlockHash::LEN + TokenAddress::LEN + PublicKey::LEN
        );
    }

    #[test]
    fn test_staged_account_key_content() {
        let state_hash = BlockHash::default();
        let token = TokenAddress::default();
        let pk = PublicKey::default();

        let key = staged_account_key(&state_hash, &token, &pk);

        // first chunk of bytes match the state hash
        assert_eq!(&key[..BlockHash::LEN], state_hash.0.as_bytes());

        // second chunk of bytes match the token
        assert_eq!(
            &key[BlockHash::LEN..][..TokenAddress::LEN],
            token.0.as_bytes()
        );

        // remaining bytes match the public key
        assert_eq!(&key[BlockHash::LEN..][TokenAddress::LEN..], pk.0.as_bytes());
    }

    #[test]
    fn test_staged_account_balance_sort_key_length() -> anyhow::Result<()> {
        let state_hash = BlockHash::default();
        let token = TokenAddress::default();
        let balance = 123456789u64;
        let pk = PublicKey::default();

        // key has the expected length
        assert_eq!(
            staged_account_balance_sort_key(&state_hash, &token, balance, &pk).len(),
            BlockHash::LEN + TokenAddress::LEN + U64_LEN + PublicKey::LEN
        );

        Ok(())
    }

    #[test]
    fn test_staged_account_balance_sort_key_content() -> anyhow::Result<()> {
        let state_hash = BlockHash::default();
        let token = TokenAddress::default();
        let balance = 987654321u64;
        let pk = PublicKey::default();

        let key = staged_account_balance_sort_key(&state_hash, &token, balance, &pk);

        // first chunk of bytes match the state hash
        assert_eq!(&key[..BlockHash::LEN], state_hash.0.as_bytes());

        // second chunk of bytes match the token
        assert_eq!(
            &key[BlockHash::LEN..][..TokenAddress::LEN],
            token.0.as_bytes()
        );

        // third chunk of bytes match the BE balance bytes
        assert_eq!(
            &key[BlockHash::LEN..][TokenAddress::LEN..][..U64_LEN],
            &balance.to_be_bytes()
        );

        // last chunk of bytes match the public key
        assert_eq!(
            &key[BlockHash::LEN..][TokenAddress::LEN..][U64_LEN..],
            pk.0.as_bytes()
        );

        Ok(())
    }
}
