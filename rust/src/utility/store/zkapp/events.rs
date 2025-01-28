use crate::{
    base::public_key::PublicKey,
    ledger::token::TokenAddress,
    utility::store::common::{token_pk_index_key, token_pk_key, U32_LEN},
};

pub fn zkapp_events_key(
    token: &TokenAddress,
    pk: &PublicKey,
    index: u32,
) -> [u8; TokenAddress::LEN + PublicKey::LEN + U32_LEN] {
    token_pk_index_key(token, pk, index)
}

pub fn zkapp_events_pk_num_key(
    token: &TokenAddress,
    pk: &PublicKey,
) -> [u8; TokenAddress::LEN + PublicKey::LEN] {
    token_pk_key(token, pk)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zkapp_events_pk_num_key() {
        let pk = PublicKey::default();
        let token = TokenAddress::default();

        let key = zkapp_events_pk_num_key(&token, &pk);

        // first token bytes
        assert_eq!(key[..TokenAddress::LEN], *token.0.as_bytes());

        // last public key bytes
        assert_eq!(key[TokenAddress::LEN..], *pk.0.as_bytes());
    }

    #[test]
    fn test_zkapp_events_key() {
        let index = 100;
        let pk = PublicKey::default();
        let token = TokenAddress::default();

        let key = zkapp_events_key(&token, &pk, index);

        // first token bytes
        assert_eq!(key[..TokenAddress::LEN], *token.0.as_bytes());

        // second public key bytes
        assert_eq!(key[TokenAddress::LEN..][..PublicKey::LEN], *pk.0.as_bytes());

        // last index BE bytes
        assert_eq!(
            key[TokenAddress::LEN..][PublicKey::LEN..],
            index.to_be_bytes()
        );
    }
}
