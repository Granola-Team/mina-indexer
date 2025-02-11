//! Token id

use super::TokenAddress;
use crate::{
    constants::MINA_TOKEN_ID, protocol::serialization_types::version_bytes::TOKEN_ID_KEY,
    utility::store::common::U64_LEN,
};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct TokenId(pub u64);

/////////////////
// conversions //
/////////////////

impl FromStr for TokenId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let blockchain_length = s.parse()?;
        Ok(Self(blockchain_length))
    }
}

impl From<TokenAddress> for TokenId {
    fn from(value: TokenAddress) -> Self {
        let bs58_bytes = value.0.as_bytes();
        let big_int = bs58::decode(bs58_bytes)
            .with_check(Some(TOKEN_ID_KEY))
            .into_vec()
            .expect("valid base58 check");

        // drop version byte
        let mut le_bytes = [0; U64_LEN];
        le_bytes.copy_from_slice(&big_int[1..=U64_LEN]);

        Self(u64::from_le_bytes(le_bytes))
    }
}

impl From<u64> for TokenId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

///////////
// serde //
///////////

impl Serialize for TokenId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        crate::utility::serde::to_str(self, serializer)
    }
}

impl<'de> Deserialize<'de> for TokenId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        crate::utility::serde::from_str(deserializer)
    }
}

/////////////
// default //
/////////////

impl Default for TokenId {
    /// MINA token id
    fn default() -> Self {
        Self(MINA_TOKEN_ID)
    }
}

/////////////
// display //
/////////////

impl Display for TokenId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::TokenId;
    use crate::ledger::token::address::TokenAddress;

    #[test]
    fn roundtrip() -> anyhow::Result<()> {
        let id = TokenId::default();

        // serialize
        let ser = serde_json::to_vec(&id)?;

        // deserialize
        let res: TokenId = serde_json::from_slice(&ser)?;

        assert_eq!(id, res);
        Ok(())
    }

    #[test]
    fn token_address_to_id() {
        let token = TokenAddress::default();
        let id = TokenId::from(token);

        assert_eq!(id, TokenId::default());
    }
}
