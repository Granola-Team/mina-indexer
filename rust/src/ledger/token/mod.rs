//! Token account, address, id, & symbol

// trait
pub mod account;

// types
mod address;
pub mod holder;
mod id;
mod symbol;

use crate::base::{amount::Amount, public_key::PublicKey};
use serde::{Deserialize, Serialize};

// re-export types
pub type TokenAddress = address::TokenAddress;
pub type TokenId = id::TokenId;
pub type TokenSymbol = symbol::TokenSymbol;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Token {
    pub token: TokenAddress,
    pub owner: Option<PublicKey>,
    pub symbol: TokenSymbol,
    pub supply: Amount,
}

#[cfg(test)]
impl quickcheck::Arbitrary for Token {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        Self {
            token: TokenAddress::arbitrary(g),
            owner: <Option<PublicKey>>::arbitrary(g),
            symbol: TokenSymbol::arbitrary(g),
            supply: Amount::arbitrary(g),
        }
    }
}
