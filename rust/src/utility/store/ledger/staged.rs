use crate::{
    base::state_hash::StateHash,
    ledger::{public_key::PublicKey, token::TokenAddress},
    utility::store::common::{balance_key_prefix, pk_key_prefix, U64_LEN},
};

/// Key format for storing staged ledger accounts by state hash
/// ```
/// {state_hash}{pk}
/// where
/// - state_hash: [StateHash::LEN] bytes
/// - token:      [TokenAddress::LEN] bytes
/// - pk:         [PublicKey::LEN] bytes
pub fn staged_account_key(
    state_hash: &StateHash,
    token: &TokenAddress,
    pk: &PublicKey,
) -> [u8; StateHash::LEN + TokenAddress::LEN + PublicKey::LEN] {
    let mut key = [0; StateHash::LEN + TokenAddress::LEN + PublicKey::LEN];

    key[..StateHash::LEN].copy_from_slice(state_hash.0.as_bytes());
    key[StateHash::LEN..][..TokenAddress::LEN].copy_from_slice(token.0.as_bytes());
    key[StateHash::LEN..][TokenAddress::LEN..].copy_from_slice(pk.0.as_bytes());
    key
}

/// Key format for sorting staged ledger accounts by balance
/// ```
/// {state_hash}{token}{balance}{pk}
/// where
/// - state_hash: [StateHash::LEN] bytes
/// - token:      [TokenAddress::LEN] bytes
/// - balance:    [u64] BE bytes
/// - pk:         [PublicKey::LEN] bytes
pub fn staged_account_balance_sort_key(
    state_hash: &StateHash,
    token: &TokenAddress,
    balance: u64,
    pk: &PublicKey,
) -> [u8; StateHash::LEN + TokenAddress::LEN + U64_LEN + PublicKey::LEN] {
    let mut key = [0; StateHash::LEN + TokenAddress::LEN + U64_LEN + PublicKey::LEN];

    key[..StateHash::LEN].copy_from_slice(state_hash.0.as_bytes());
    key[StateHash::LEN..][..TokenAddress::LEN].copy_from_slice(token.0.as_bytes());
    key[StateHash::LEN..][TokenAddress::LEN..][..U64_LEN].copy_from_slice(&balance.to_be_bytes());
    key[StateHash::LEN..][TokenAddress::LEN..][U64_LEN..].copy_from_slice(pk.0.as_bytes());
    key
}

/// Split [staged_account_balance_sort_key] into constituent parts
pub fn split_staged_account_balance_sort_key(
    key: &[u8],
) -> Option<(StateHash, TokenAddress, u64, PublicKey)> {
    if key.len() == StateHash::LEN + TokenAddress::LEN + U64_LEN + PublicKey::LEN {
        let state_hash = StateHash::from_bytes(&key[..StateHash::LEN]).expect("block hash");
        let token = TokenAddress::from_bytes(key[StateHash::LEN..][..TokenAddress::LEN].to_vec())
            .expect("token address");

        let balance = balance_key_prefix(&key[StateHash::LEN..][TokenAddress::LEN..]);
        let pk = pk_key_prefix(&key[StateHash::LEN..][TokenAddress::LEN..][U64_LEN..]);

        return Some((state_hash, token, balance, pk));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{base::state_hash::StateHash, ledger::public_key::PublicKey};

    #[test]
    fn test_staged_account_key_length() {
        let state_hash = StateHash::default();
        let token = TokenAddress::default();
        let pk = PublicKey::default();

        // key has the expected length
        assert_eq!(
            staged_account_key(&state_hash, &token, &pk).len(),
            StateHash::LEN + TokenAddress::LEN + PublicKey::LEN
        );
    }

    #[test]
    fn test_staged_account_key_content() {
        let state_hash = StateHash::default();
        let token = TokenAddress::default();
        let pk = PublicKey::default();

        let key = staged_account_key(&state_hash, &token, &pk);

        // first chunk of bytes match the state hash
        assert_eq!(&key[..StateHash::LEN], state_hash.0.as_bytes());

        // second chunk of bytes match the token
        assert_eq!(
            &key[StateHash::LEN..][..TokenAddress::LEN],
            token.0.as_bytes()
        );

        // remaining bytes match the public key
        assert_eq!(&key[StateHash::LEN..][TokenAddress::LEN..], pk.0.as_bytes());
    }

    #[test]
    fn test_staged_account_balance_sort_key_length() -> anyhow::Result<()> {
        let state_hash = StateHash::default();
        let token = TokenAddress::default();
        let balance = 123456789u64;
        let pk = PublicKey::default();

        // key has the expected length
        assert_eq!(
            staged_account_balance_sort_key(&state_hash, &token, balance, &pk).len(),
            StateHash::LEN + TokenAddress::LEN + U64_LEN + PublicKey::LEN
        );

        Ok(())
    }

    #[test]
    fn test_staged_account_balance_sort_key_content() -> anyhow::Result<()> {
        let state_hash = StateHash::default();
        let token = TokenAddress::default();
        let balance = 987654321u64;
        let pk = PublicKey::default();

        let key = staged_account_balance_sort_key(&state_hash, &token, balance, &pk);

        // first chunk of bytes match the state hash
        assert_eq!(&key[..StateHash::LEN], state_hash.0.as_bytes());

        // second chunk of bytes match the token
        assert_eq!(
            &key[StateHash::LEN..][..TokenAddress::LEN],
            token.0.as_bytes()
        );

        // third chunk of bytes match the BE balance bytes
        assert_eq!(
            &key[StateHash::LEN..][TokenAddress::LEN..][..U64_LEN],
            &balance.to_be_bytes()
        );

        // last chunk of bytes match the public key
        assert_eq!(
            &key[StateHash::LEN..][TokenAddress::LEN..][U64_LEN..],
            pk.0.as_bytes()
        );

        Ok(())
    }
}
