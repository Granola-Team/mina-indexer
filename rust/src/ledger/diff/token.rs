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
    pub token: TokenAddress,
    pub owner: PublicKey,
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

        for TokenDiff { owner, diff, .. } in diffs {
            if token.token != TokenAddress::default() {
                token.owner = Some(owner.to_owned());
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
                owner: public_key,
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
            owner: public_key,
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
