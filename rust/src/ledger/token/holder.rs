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
    pub kind: TokenHolderKind,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TokenHolderKind {
    #[default]
    Credit,
    Debit,
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
            Supply(amt) => {
                let amt_abs = amt.unsigned_abs();

                if self.kind == TokenHolderKind::Credit && *amt < 0 {
                    if amt_abs > self.balance.0 {
                        self.kind = TokenHolderKind::Debit;
                        self.balance = (amt_abs - self.balance.0).into()
                    }
                } else if self.kind == TokenHolderKind::Debit && *amt > 0 {
                    if amt_abs > self.balance.0 {
                        self.kind = TokenHolderKind::Credit;
                        self.balance = (amt_abs - self.balance.0).into()
                    }
                } else {
                    self.balance += amt_abs;
                }
            }
            Owner(owner) => self.public_key = owner.to_owned(),
            _ => (),
        }
    }

    pub fn unapply(&mut self, diff: &TokenDiff) {
        if let TokenDiffType::Supply(amt) = &diff.diff {
            let amt_abs = amt.unsigned_abs();

            if self.kind == TokenHolderKind::Credit && *amt > 0 {
                if amt_abs > self.balance.0 {
                    self.kind = TokenHolderKind::Debit;
                    self.balance = (amt_abs - self.balance.0).into()
                }
            } else if self.kind == TokenHolderKind::Debit && *amt < 0 {
                if amt_abs > self.balance.0 {
                    self.kind = TokenHolderKind::Credit;
                    self.balance = (amt_abs - self.balance.0).into()
                }
            } else {
                self.balance -= amt_abs;
            }
        }
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for TokenHolderKind {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        if bool::arbitrary(g) {
            Self::Credit
        } else {
            Self::Debit
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
            kind: TokenHolderKind::arbitrary(g),
        }
    }
}
