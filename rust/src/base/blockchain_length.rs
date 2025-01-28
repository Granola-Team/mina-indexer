//! Indexer blockchain length type

use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct BlockchainLength(pub u32);

/////////////////
// conversions //
/////////////////

impl From<u32> for BlockchainLength {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl FromStr for BlockchainLength {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let blockchain_length = s.parse()?;
        Ok(Self(blockchain_length))
    }
}

///////////
// serde //
///////////

impl Serialize for BlockchainLength {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        crate::utility::serde::to_str(self, serializer)
    }
}

impl<'de> Deserialize<'de> for BlockchainLength {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        crate::utility::serde::from_str(deserializer)
    }
}

/////////////
// display //
/////////////

impl Display for BlockchainLength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::BlockchainLength;

    #[test]
    fn roundtrip() -> anyhow::Result<()> {
        let len = BlockchainLength::default();
        let len_str = len.to_string();

        // serialize
        let ser = serde_json::to_vec(&len)?;

        // deserialize
        let res: BlockchainLength = serde_json::from_slice(&ser)?;

        // roundtrip
        assert_eq!(len, res);

        // same serialization as string
        assert_eq!(ser, serde_json::to_vec(&len_str)?);

        Ok(())
    }
}
