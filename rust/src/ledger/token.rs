use super::{account::Amount, public_key::PublicKey};
use crate::constants::MINA_TOKEN_ADDRESS;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Token {
    address: TokenAddress,
    owner: Option<PublicKey>,
    symbol: TokenSymbol,
    supply: Amount,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenSymbol(pub String);

/// Also referred to as `TokenId`
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TokenAddress(pub String);

impl TokenAddress {
    pub const LEN: usize = 50;

    pub fn from_bytes(bytes: Vec<u8>) -> anyhow::Result<Self> {
        String::from_utf8(bytes)
            .map(Self)
            .map_err(anyhow::Error::new)
    }

    pub fn new<T>(token: T) -> Option<Self>
    where
        T: Into<String>,
    {
        let token: String = token.into();
        if token.len() == Self::LEN {
            return Some(Self(token));
        }

        None
    }

    /// Used to surpass the [TokenAddress] bytes in a db key
    pub fn upper_bound() -> [u8; TokenAddress::LEN] {
        [u8::MAX; TokenAddress::LEN]
    }
}

impl From<u64> for TokenAddress {
    fn from(value: u64) -> Self {
        match value {
            1 => Self::default(),
            _ => todo!("unknown token id {value}"),
        }
    }
}

impl From<TokenAddress> for u64 {
    fn from(value: TokenAddress) -> Self {
        if value.0 == MINA_TOKEN_ADDRESS {
            1
        } else {
            todo!("unknown token address {value}")
        }
    }
}

impl std::fmt::Display for TokenAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::default::Default for TokenAddress {
    /// MINA token address
    fn default() -> Self {
        Self(MINA_TOKEN_ADDRESS.into())
    }
}

impl std::default::Default for TokenSymbol {
    /// MINA token symbol
    fn default() -> Self {
        Self("MINA".into())
    }
}
