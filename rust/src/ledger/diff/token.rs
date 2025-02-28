//! Token diff type

use super::{
    account::{ZkappPaymentDiff, ZkappTokenSymbolDiff},
    TokenAddress,
};
use crate::{
    base::public_key::PublicKey,
    ledger::{
        diff::account::{PaymentDiff, UpdateType},
        token::{Token, TokenSymbol},
    },
};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TokenDiff {
    pub public_key: PublicKey,
    pub token: TokenAddress,
    pub diff: TokenDiffType,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TokenDiffType {
    Supply(i64),
    Owner(PublicKey),
    Symbol(TokenSymbol),
}

//////////
// impl //
//////////

impl TokenDiff {
    pub fn aggregate(token: Token, diffs: &[&Self]) -> Token {
        let mut token = token;

        for TokenDiff {
            public_key, diff, ..
        } in diffs
        {
            if token.token != TokenAddress::default() {
                token.owner = Some(public_key.to_owned());
            }

            use TokenDiffType::*;
            match diff {
                Supply(amt) => token.supply += *amt,
                Owner(owner) => token.owner = Some(owner.to_owned()),
                Symbol(symbol) => token.symbol = symbol.to_owned(),
            }
        }

        token
    }
}

impl TokenDiffType {
    pub fn amount(&self) -> u64 {
        match self {
            Self::Supply(amt) => amt.unsigned_abs(),
            _ => 0,
        }
    }
}

/////////////////
// conversions //
/////////////////

impl From<ZkappPaymentDiff> for Option<TokenDiff> {
    fn from(value: ZkappPaymentDiff) -> Self {
        use ZkappPaymentDiff::*;

        if let Payment {
            payment:
                PaymentDiff {
                    update_type,
                    public_key,
                    amount,
                    token,
                },
            ..
        } = value
        {
            return Some(TokenDiff {
                token,
                public_key,
                diff: TokenDiffType::Supply({
                    let amt_i64 = amount.0 as i64;
                    if update_type == UpdateType::Credit {
                        amt_i64
                    } else {
                        -amt_i64
                    }
                }),
            });
        }

        None
    }
}

impl From<ZkappTokenSymbolDiff> for TokenDiff {
    fn from(value: ZkappTokenSymbolDiff) -> Self {
        let ZkappTokenSymbolDiff {
            token,
            public_key,
            token_symbol,
            ..
        } = value;

        Self {
            token,
            public_key,
            diff: TokenDiffType::Symbol(token_symbol),
        }
    }
}

//////////////
// defaults //
//////////////

impl std::default::Default for TokenDiffType {
    fn default() -> Self {
        Self::Supply(0)
    }
}

///////////////
// arbitrary //
///////////////

#[cfg(test)]
impl quickcheck::Arbitrary for TokenDiffType {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        match u8::arbitrary(g) % 3 {
            0 => Self::Owner(PublicKey::arbitrary(g)),
            1 => Self::Supply(i64::arbitrary(g)),
            2 => Self::Symbol(TokenSymbol::arbitrary(g)),
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for TokenDiff {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let pk = PublicKey::arbitrary(g);

        Self {
            public_key: pk.to_owned(),
            token: TokenAddress::arbitrary(g),
            diff: {
                match u8::arbitrary(g) % 3 {
                    0 => TokenDiffType::Owner(pk),
                    1 => TokenDiffType::Supply(i64::arbitrary(g)),
                    2 => TokenDiffType::Symbol(TokenSymbol::arbitrary(g)),
                    _ => unreachable!(),
                }
            },
        }
    }
}

#[cfg(test)]
impl TokenDiff {
    fn arbitrary_with_pk(g: &mut quickcheck::Gen, pk: PublicKey) -> Self {
        use quickcheck::Arbitrary;

        Self {
            public_key: pk.to_owned(),
            token: TokenAddress::arbitrary(g),
            diff: {
                match u8::arbitrary(g) % 3 {
                    0 => TokenDiffType::Owner(pk),
                    1 => TokenDiffType::Supply(i8::arbitrary(g) as i64),
                    2 => TokenDiffType::Symbol(TokenSymbol::arbitrary(g)),
                    _ => unreachable!(),
                }
            },
        }
    }

    pub fn arbitrary_with_address_pk(
        g: &mut quickcheck::Gen,
        token: TokenAddress,
        pk: PublicKey,
    ) -> Self {
        let mut diff = Self::arbitrary_with_pk(g, pk);
        diff.token = token;

        diff
    }
}
