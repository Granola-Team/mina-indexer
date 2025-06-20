//! Token address

use super::TokenId;
use crate::{
    constants::MINA_TOKEN_ADDRESS, protocol::serialization_types::version_bytes::TOKEN_ID_KEY,
    utility::store::common::U64_LEN,
};
use anyhow::bail;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
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

///////////
// check //
///////////

impl crate::base::check::Check for Option<TokenAddress> {
    fn check(&self, other: &Self) -> bool {
        match (self.as_ref(), other.as_ref()) {
            (Some(self_token), Some(token)) => {
                let check = self_token != token;
                if check {
                    log::error!("Mismatching tokens {} {}", self_token, token)
                }

                check
            }
            (Some(token), _) | (_, Some(token)) => {
                let check = token.0 != MINA_TOKEN_ADDRESS;
                if check {
                    log::error!("Mismatching token {}", token)
                }

                check
            }
            _ => false,
        }
    }
}

///////////
// serde //
///////////

impl<'de> Deserialize<'de> for TokenAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        crate::utility::serde::from_str(deserializer)
    }
}

/////////////////
// conversions //
/////////////////

impl std::str::FromStr for TokenAddress {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(token) = Self::new(s) {
            return Ok(token);
        }

        bail!("Invalid token address: {s}")
    }
}

impl From<TokenId> for TokenAddress {
    fn from(value: TokenId) -> Self {
        let mut big_int = [0; 32];
        let le_bytes = value.0.to_le_bytes();

        // big int LE bytes
        big_int[..U64_LEN].copy_from_slice(&le_bytes);
        Self(
            bs58::encode(&big_int[..])
                .with_check_version(TOKEN_ID_KEY)
                .into_string(),
        )
    }
}

/////////////
// default //
/////////////

impl std::default::Default for TokenAddress {
    /// MINA token address
    fn default() -> Self {
        Self(MINA_TOKEN_ADDRESS.into())
    }
}

/////////////
// display //
/////////////

impl std::fmt::Display for TokenAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for TokenAddress {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let mut bytes = [0u8; 32];
        for b in &mut bytes {
            *b = u8::arbitrary(g);
        }

        Self(
            bs58::encode(&bytes)
                .with_check_version(TOKEN_ID_KEY)
                .into_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::token::id::TokenId;
    use quickcheck::{Arbitrary, Gen};

    #[test]
    fn arbitrary_token_is_valid() {
        let token = TokenAddress::arbitrary(&mut Gen::new(1000));

        assert_eq!(token.0.len(), TokenAddress::LEN);
        assert!(bs58::decode(&token.0.as_bytes())
            .with_check(Some(TOKEN_ID_KEY))
            .into_vec()
            .is_ok());
    }

    #[test]
    fn roundtrip() -> anyhow::Result<()> {
        let token = TokenAddress::default();

        // serialize
        let ser = serde_json::to_vec(&token)?;

        // deserialize
        let res: TokenAddress = serde_json::from_slice(&ser)?;

        assert_eq!(token, res);
        Ok(())
    }

    #[test]
    fn id_to_token_address() {
        let token = TokenAddress::from(TokenId::default());
        assert_eq!(token.0, MINA_TOKEN_ADDRESS);
    }
}
