//! Token holder type

use super::TokenAddress;
use crate::base::{amount::Amount, public_key::PublicKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenHolder {
    pub public_key: PublicKey,
    pub token: TokenAddress,
    pub balance: Amount,
}

#[cfg(test)]
impl quickcheck::Arbitrary for TokenHolder {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        Self {
            public_key: PublicKey::arbitrary(g),
            token: TokenAddress::arbitrary(g),
            balance: Amount::arbitrary(g),
        }
    }
}
