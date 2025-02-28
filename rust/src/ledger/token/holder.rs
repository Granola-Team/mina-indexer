//! Token holder type

use super::TokenAddress;
use crate::{
    base::{amount::Amount, public_key::PublicKey},
    ledger::diff::token::{TokenDiff, TokenDiffType},
};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenHolder {
    pub public_key: PublicKey,
    pub token: TokenAddress,
    pub balance: Amount,
}

//////////
// impl //
//////////

impl TokenHolder {
    pub fn new(token: TokenAddress, public_key: PublicKey) -> Self {
        Self {
            token,
            public_key,
            ..Default::default()
        }
    }

    pub fn apply(&mut self, diff: &TokenDiff) {
        use TokenDiffType::*;

        match &diff.diff {
            Supply(amt) => self.balance += *amt,
            Owner(owner) => self.public_key = owner.to_owned(),
            _ => (),
        }
    }

    pub fn unapply(&mut self, diff: &TokenDiff) {
        if let TokenDiffType::Supply(amt) = &diff.diff {
            self.balance -= *amt;
        }
    }
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
