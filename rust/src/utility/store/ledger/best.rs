use crate::{
    ledger::{public_key::PublicKey, token::TokenAddress},
    utility::store::{balance_key_prefix, pk_key_prefix, U64_LEN},
};

/// Key format for storing best ledger accounts
/// ```
/// {token}{pk}
/// where
/// - token: [TokenAddress::LEN] bytes
/// - pk:    [PublicKey::LEN] bytes
pub fn best_account_key(
    token: &TokenAddress,
    pk: &PublicKey,
) -> [u8; TokenAddress::LEN + PublicKey::LEN] {
    let mut key = [0; TokenAddress::LEN + PublicKey::LEN];

    key[..TokenAddress::LEN].copy_from_slice(token.0.as_bytes());
    key[TokenAddress::LEN..].copy_from_slice(pk.0.as_bytes());
    key
}

/// Key format for sorting best ledger accounts
/// ```
/// {token}{pk}
/// where
/// - token:   [TokenAddress::LEN] bytes
/// - balance: [u64] BE bytes
/// - pk:      [PublicKey::LEN] bytes
pub fn best_account_sort_key(
    token: &TokenAddress,
    balance: u64,
    pk: &PublicKey,
) -> [u8; TokenAddress::LEN + U64_LEN + PublicKey::LEN] {
    let mut key = [0; TokenAddress::LEN + U64_LEN + PublicKey::LEN];

    key[..TokenAddress::LEN].copy_from_slice(token.0.as_bytes());
    key[TokenAddress::LEN..][..U64_LEN].copy_from_slice(&balance.to_be_bytes());
    key[TokenAddress::LEN..][U64_LEN..].copy_from_slice(pk.0.as_bytes());
    key
}

/// Split [best_account_sort_key] into constituent parts
pub fn split_best_account_sort_key(key: &[u8]) -> Option<(TokenAddress, u64, PublicKey)> {
    if key.len() == TokenAddress::LEN + U64_LEN + PublicKey::LEN {
        let token =
            TokenAddress::from_bytes(key[..TokenAddress::LEN].to_vec()).expect("token address");

        let balance = balance_key_prefix(&key[TokenAddress::LEN..]);
        let pk = pk_key_prefix(&key[TokenAddress::LEN..][U64_LEN..]);

        return Some((token, balance, pk));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn best_account_key_content() {
        let token = TokenAddress::default();
        let pk = PublicKey::default();

        let key = best_account_key(&token, &pk);

        // first chunk of bytes match the token
        assert_eq!(&key[..TokenAddress::LEN], token.0.as_bytes());

        // remaining bytes match the public key
        assert_eq!(&key[TokenAddress::LEN..], pk.0.as_bytes());
    }

    #[test]
    fn best_account_sort_key_content() {
        let token = TokenAddress::default();
        let balance = 1000;
        let pk = PublicKey::default();

        let key = best_account_sort_key(&token, balance, &pk);

        // first chunk of bytes match the token
        assert_eq!(&key[..TokenAddress::LEN], token.0.as_bytes());

        // second chunk of bytes match the token
        assert_eq!(&key[TokenAddress::LEN..][..U64_LEN], &balance.to_be_bytes());

        // remaining bytes match the public key
        assert_eq!(&key[TokenAddress::LEN..][U64_LEN..], pk.0.as_bytes());
    }

    #[test]
    fn best_account_key_split() {
        let token = TokenAddress::default();
        let balance = 1000;
        let pk = PublicKey::default();

        let key = best_account_sort_key(&token, balance, &pk);

        match split_best_account_sort_key(&key) {
            Some((key_token, key_balance, key_pk)) => {
                assert_eq!(key_token, token);
                assert_eq!(key_balance, balance);
                assert_eq!(key_pk, pk);
            }
            _ => panic!("Invalid split_best_account_sort_key"),
        }
    }
}
