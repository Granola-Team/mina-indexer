//! Token account, address, id, & symbol

// trait
pub mod account;

// types
mod address;
pub mod holder;
mod id;
mod symbol;

use super::diff::token::{TokenDiff, TokenDiffType};
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

//////////
// impl //
//////////

impl Token {
    /// Create a new token with the specified address
    pub fn new(token: TokenAddress) -> Self {
        Self {
            token,
            ..Default::default()
        }
    }

    /// Create a new token with the specified address & owner
    pub fn new_with_owner(token: TokenAddress, owner: PublicKey) -> Self {
        Self {
            token,
            owner: Some(owner),
            ..Default::default()
        }
    }

    /// Apply a token diff to the token
    pub fn apply(&mut self, diff: TokenDiff) {
        use TokenDiffType::*;

        match diff.diff {
            Supply(amt) => self.supply += amt,
            Owner(owner) => self.owner = Some(owner),
            Symbol(symbol) => self.symbol = symbol,
        }
    }

    /// Unapply a token diff to the token
    pub fn unapply(&mut self, diff: TokenDiff) {
        use TokenDiffType::*;

        match diff.diff {
            Supply(amt) => self.supply -= amt,
            Owner(owner) => self.owner = Some(owner),
            Symbol(symbol) => self.symbol = symbol,
        }
    }
}

impl std::ops::AddAssign<TokenDiff> for Token {
    fn add_assign(&mut self, rhs: TokenDiff) {
        assert_eq!(
            self.token, rhs.token,
            "diff & token addresses must match to add assign"
        );

        use TokenDiffType::*;
        match &rhs.diff {
            Supply(amt) => self.supply += *amt,
            Owner(owner) => self.owner = Some(owner.to_owned()),
            Symbol(symbol) => self.symbol = symbol.to_owned(),
        }
    }
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
