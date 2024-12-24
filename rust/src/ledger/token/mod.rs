pub mod symbol;

use super::{amount::Amount, public_key::PublicKey};
use crate::{
    constants::MINA_TOKEN_ADDRESS, protocol::serialization_types::version_bytes::TOKEN_ID_KEY,
    utility::store::U64_LEN,
};
use anyhow::bail;
use serde::{Deserialize, Serialize};
use symbol::TokenSymbol;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Token {
    address: TokenAddress,
    owner: Option<PublicKey>,
    symbol: TokenSymbol,
    supply: Amount,
}

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

impl std::str::FromStr for TokenAddress {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(token) = Self::new(s) {
            return Ok(token);
        }

        bail!("Invalid token address: {s}")
    }
}

impl From<u64> for TokenAddress {
    fn from(value: u64) -> Self {
        let mut big_int = [0; 32];
        let le_bytes = value.to_le_bytes();

        // big int LE bytes
        big_int[..U64_LEN].copy_from_slice(&le_bytes);
        Self(
            bs58::encode(&big_int[..])
                .with_check_version(TOKEN_ID_KEY)
                .into_string(),
        )
    }
}

impl From<TokenAddress> for u64 {
    fn from(value: TokenAddress) -> Self {
        let bs58_bytes = value.0.as_bytes();
        let big_int = bs58::decode(bs58_bytes)
            .with_check(Some(TOKEN_ID_KEY))
            .into_vec()
            .expect("valid base58 check");

        // drop version byte
        let mut le_bytes = [0; U64_LEN];
        le_bytes.copy_from_slice(&big_int[1..=U64_LEN]);

        u64::from_le_bytes(le_bytes)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::MINA_TOKEN_ID;

    #[test]
    fn u64_to_token_address() {
        let token = TokenAddress::from(MINA_TOKEN_ID);
        assert_eq!(token.0, MINA_TOKEN_ADDRESS);
    }

    #[test]
    fn token_address_to_u64() {
        let token = TokenAddress::default();
        let id = u64::from(token);

        assert_eq!(id, MINA_TOKEN_ID);
    }
}
