//! Indexer state hash type

use crate::protocol::serialization_types::{
    common::{Base58EncodableVersionedType, HashV1},
    version_bytes,
};
use anyhow::bail;
use serde::{Deserialize, Serialize};

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize)]
pub struct StateHash(pub String);

impl StateHash {
    pub const LEN: usize = 52;
    pub const PREFIX: &'static str = "3N";

    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let res = String::from_utf8(bytes.to_vec())?;

        if Self::is_valid(&res) {
            return Ok(Self(res));
        }

        bail!("Invalid state hash from bytes")
    }

    pub fn from_bytes_or_panic(bytes: Vec<u8>) -> Self {
        Self::from_bytes(&bytes).expect("block state hash bytes")
    }

    pub fn from_hashv1(hashv1: HashV1) -> Self {
        let versioned: Base58EncodableVersionedType<{ version_bytes::STATE_HASH }, _> =
            hashv1.into();
        Self(versioned.to_base58_string().expect("block state hash"))
    }

    pub fn to_bytes(self) -> [u8; StateHash::LEN] {
        let mut res = [0u8; StateHash::LEN];

        res.copy_from_slice(self.0.as_bytes());
        res
    }

    pub fn is_valid(input: &str) -> bool {
        input.starts_with(StateHash::PREFIX) && input.len() == StateHash::LEN
    }
}

///////////
// serde //
///////////

impl<'de> Deserialize<'de> for StateHash {
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

impl std::str::FromStr for StateHash {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if Self::is_valid(s) {
            Ok(Self(s.to_string()))
        } else {
            bail!("Invalid state hash: {s}")
        }
    }
}

impl<T> From<T> for StateHash
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

#[cfg(test)]
mod tests {
    use super::StateHash;

    #[test]
    fn roundtrip() -> anyhow::Result<()> {
        let hash = StateHash::default();
        let hash_str = hash.to_string();

        // serialize
        let ser = serde_json::to_vec(&hash)?;

        // deserialize
        let res: StateHash = serde_json::from_slice(&ser)?;

        // roundtrip
        assert_eq!(hash, res);

        // same serialization as string
        assert_eq!(ser, serde_json::to_vec(&hash_str)?);

        Ok(())
    }
}
