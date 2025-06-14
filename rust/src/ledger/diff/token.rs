//! Token diff type

use super::{
    account::zkapp::{ZkappPaymentDiff, ZkappTokenSymbolDiff},
    TokenAddress,
};
use crate::{
    base::public_key::PublicKey,
    ledger::{
        diff::account::{PaymentDiff, UpdateType},
        token::TokenSymbol,
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
                    ..
                },
            ..
        } = value
        {
            return Some(TokenDiff {
                public_key,
                token: token.unwrap_or_default(),
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
    fn arbitrary_with_pk_max_supply(
        g: &mut quickcheck::Gen,
        pk: PublicKey,
        amount: crate::base::amount::Amount,
    ) -> Self {
        use quickcheck::Arbitrary;

        Self {
            public_key: pk.to_owned(),
            token: TokenAddress::arbitrary(g),
            diff: {
                match u8::arbitrary(g) % 3 {
                    0 => TokenDiffType::Owner(pk),
                    1 => {
                        let supply_u8 = u8::arbitrary(g);

                        let supply = if supply_u8 as u64 <= amount.0 {
                            if bool::arbitrary(g) {
                                -(supply_u8 as i64)
                            } else {
                                supply_u8 as i64
                            }
                        } else {
                            supply_u8 as i64
                        };
                        TokenDiffType::Supply(supply)
                    }
                    2 => TokenDiffType::Symbol(TokenSymbol::arbitrary(g)),
                    _ => unreachable!(),
                }
            },
        }
    }

    pub fn arbitrary_with_address_pk_max_supply(
        g: &mut quickcheck::Gen,
        token: TokenAddress,
        pk: PublicKey,
        amount: crate::base::amount::Amount,
    ) -> Self {
        let mut diff = Self::arbitrary_with_pk_max_supply(g, pk, amount);
        diff.token = token;

        diff
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        block::precomputed::{PcbVersion, PrecomputedBlock},
        ledger::{
            diff::{
                token::{TokenDiff, TokenDiffType},
                LedgerDiff,
            },
            token::TokenAddress,
        },
    };
    use std::path::PathBuf;

    #[test]
    fn token_diffs() -> anyhow::Result<()> {
        let path = PathBuf::from("./tests/data/misc_blocks/mainnet-360930-3NL3mVAEwJuBS8F3fMWBZZRjQC4JBzdGTD7vN5SqizudnkPKsRyi.json");
        let pcb = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;
        let diff = LedgerDiff::from_precomputed(&pcb);

        let token_diffs = diff.token_diffs;
        let expect = vec![
            TokenDiff {
                public_key: "B62qo69VLUPMXEC6AFWRgjdTEGsA3xKvqeU5CgYm3jAbBJL7dTvaQkv".into(),
                token: TokenAddress::default(),
                diff: TokenDiffType::Supply(-1000000000),
            },
            TokenDiff {
                public_key: "B62qnzkHunByjReoEwMKCJ9HQxZP2MSYcUe8Lfesy4SpufxWp3viNFT".into(),
                token: TokenAddress::default(),
                diff: TokenDiffType::Supply(0),
            },
            TokenDiff {
                public_key: "B62qo69VLUPMXEC6AFWRgjdTEGsA3xKvqeU5CgYm3jAbBJL7dTvaQkv".into(),
                token: TokenAddress::default(),
                diff: TokenDiffType::Supply(0),
            },
            TokenDiff {
                public_key: "B62qo69VLUPMXEC6AFWRgjdTEGsA3xKvqeU5CgYm3jAbBJL7dTvaQkv".into(),
                token: TokenAddress::default(),
                diff: TokenDiffType::Supply(-19000000000),
            },
            TokenDiff {
                public_key: "B62qq7ecvBQZQK68dwstL27888NEKZJwNXNFjTyu3xpQcfX5UBivCU6".into(),
                token: TokenAddress::default(),
                diff: TokenDiffType::Supply(19000000000),
            },
            TokenDiff {
                public_key: "B62qnVLedrzTUMZME91WKbNw3qJ3hw7cc5PeK6RR3vH7RTFTsVbiBj4".into(),
                token: TokenAddress::new("xosVXFFDvDiKvHSDAaHvrTSRtoa5Graf2J7LM5Smb4GNTrT2Hn")
                    .unwrap(),
                diff: TokenDiffType::Supply(0),
            },
            TokenDiff {
                public_key: "B62qnVLedrzTUMZME91WKbNw3qJ3hw7cc5PeK6RR3vH7RTFTsVbiBj4".into(),
                token: TokenAddress::new("xosVXFFDvDiKvHSDAaHvrTSRtoa5Graf2J7LM5Smb4GNTrT2Hn")
                    .unwrap(),
                diff: TokenDiffType::Supply(1000000000),
            },
        ];

        for (n, x) in expect.iter().enumerate() {
            assert_eq!(token_diffs[n], *x, "n = {n}");
        }

        assert_eq!(token_diffs, expect);
        Ok(())
    }
}
