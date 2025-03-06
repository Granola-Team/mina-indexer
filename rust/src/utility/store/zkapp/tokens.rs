//! Zkapp tokens store key helpers

use crate::{
    base::public_key::PublicKey,
    ledger::token::TokenAddress,
    utility::store::common::{U32_LEN, U64_LEN},
};

/// Use with [zkapp_tokens_holder_cf]
pub fn zkapp_tokens_holder_key(
    token: &TokenAddress,
    index: u32,
) -> [u8; TokenAddress::LEN + U32_LEN] {
    let mut key = [0; TokenAddress::LEN + U32_LEN];

    key[..TokenAddress::LEN].copy_from_slice(token.0.as_bytes());
    key[TokenAddress::LEN..].copy_from_slice(&index.to_be_bytes());

    key
}

/// Use with [zkapp_tokens_holder_index_cf]
pub fn zkapp_tokens_holder_index_key(
    token: &TokenAddress,
    pk: &PublicKey,
) -> [u8; TokenAddress::LEN + PublicKey::LEN] {
    let mut key = [0; TokenAddress::LEN + PublicKey::LEN];

    key[..TokenAddress::LEN].copy_from_slice(token.0.as_bytes());
    key[TokenAddress::LEN..].copy_from_slice(pk.0.as_bytes());

    key
}

/// Use with [zkapp_tokens_historical_diffs_cf]
pub fn zkapp_tokens_historical_diffs_key(
    token: &TokenAddress,
    index: u32,
) -> [u8; TokenAddress::LEN + U32_LEN] {
    zkapp_tokens_holder_key(token, index)
}

/// Use with [zkapp_tokens_historical_owners_cf]
pub fn zkapp_tokens_historical_owners_key(
    token: &TokenAddress,
    index: u32,
) -> [u8; TokenAddress::LEN + U32_LEN] {
    zkapp_tokens_holder_key(token, index)
}

/// Use with [zkapp_tokens_historical_symbols_cf]
pub fn zkapp_tokens_historical_symbols_key(
    token: &TokenAddress,
    index: u32,
) -> [u8; TokenAddress::LEN + U32_LEN] {
    zkapp_tokens_holder_key(token, index)
}

/// Use with [zkapp_tokens_historical_supplies_cf]
pub fn zkapp_tokens_historical_supplies_key(
    token: &TokenAddress,
    index: u32,
) -> [u8; TokenAddress::LEN + U32_LEN] {
    zkapp_tokens_holder_key(token, index)
}

/// Use with [zkapp_tokens_pk_cf]
pub fn zkapp_tokens_pk_key(pk: &PublicKey, index: u32) -> [u8; PublicKey::LEN + U32_LEN] {
    let mut key = [0; PublicKey::LEN + U32_LEN];

    key[..PublicKey::LEN].copy_from_slice(pk.0.as_bytes());
    key[PublicKey::LEN..].copy_from_slice(&index.to_be_bytes());

    key
}

/// Use with [zkapp_tokens_pk_index_cf]
pub fn zkapp_tokens_pk_index_key(
    token: &TokenAddress,
    pk: &PublicKey,
) -> [u8; TokenAddress::LEN + PublicKey::LEN] {
    zkapp_tokens_holder_index_key(token, pk)
}

/// Use with [zkapp_tokens_historical_pk_diffs_cf]
pub fn zkapp_tokens_historical_pk_diffs_key(
    pk: &PublicKey,
    index: u32,
) -> [u8; PublicKey::LEN + U32_LEN] {
    zkapp_tokens_pk_key(pk, index)
}

/// Use with [zkapp_tokens_supply_sort_cf]
pub fn zkapp_tokens_supply_sort_key(
    supply: u64,
    token: &TokenAddress,
) -> [u8; U64_LEN + TokenAddress::LEN] {
    let mut key = [0; U64_LEN + TokenAddress::LEN];

    key[..U64_LEN].copy_from_slice(&supply.to_be_bytes());
    key[U64_LEN..].copy_from_slice(token.0.as_bytes());

    key
}

/// Use with [zkapp_tokens_supply_sort_cf]
pub fn zkapp_tokens_supply_sort_key_token(bytes: &[u8]) -> TokenAddress {
    TokenAddress::from_bytes(bytes[U64_LEN..].to_vec())
        .expect("token address from zkapp_tokens_supply_sort_key")
}

#[cfg(test)]
mod tests {
    use crate::{base::public_key::PublicKey, ledger::token::TokenAddress};
    use quickcheck::{Arbitrary, Gen};

    #[test]
    fn zkapp_tokens_holder_key() -> anyhow::Result<()> {
        let g = &mut Gen::new(1000);

        let token = TokenAddress::arbitrary(g);
        let index = u32::arbitrary(g);

        let key = super::zkapp_tokens_holder_key(&token, index);
        assert_eq!(key[..TokenAddress::LEN], *token.0.as_bytes());
        assert_eq!(key[TokenAddress::LEN..], index.to_be_bytes());

        Ok(())
    }

    #[test]
    fn zkapp_tokens_pk_key() -> anyhow::Result<()> {
        let g = &mut Gen::new(1000);

        let pk = PublicKey::arbitrary(g);
        let index = u32::arbitrary(g);

        let key = super::zkapp_tokens_pk_key(&pk, index);
        assert_eq!(key[..PublicKey::LEN], *pk.0.as_bytes());
        assert_eq!(key[PublicKey::LEN..], index.to_be_bytes());

        Ok(())
    }

    #[test]
    fn zkapp_tokens_pk_index_key() -> anyhow::Result<()> {
        let g = &mut Gen::new(1000);

        let token = TokenAddress::arbitrary(g);
        let pk = PublicKey::arbitrary(g);

        let key = super::zkapp_tokens_pk_index_key(&token, &pk);
        assert_eq!(key[..TokenAddress::LEN], *token.0.as_bytes());
        assert_eq!(key[TokenAddress::LEN..], *pk.0.as_bytes());

        Ok(())
    }

    #[test]
    fn zkapp_tokens_historical_owners_key() -> anyhow::Result<()> {
        let g = &mut Gen::new(1000);

        let token = TokenAddress::arbitrary(g);
        let index = u32::arbitrary(g);

        let key = super::zkapp_tokens_historical_owners_key(&token, index);
        assert_eq!(key[..TokenAddress::LEN], *token.0.as_bytes());
        assert_eq!(key[TokenAddress::LEN..], index.to_be_bytes());

        Ok(())
    }
}
