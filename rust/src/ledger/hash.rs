use crate::protocol::serialization_types::{
    common::{Base58EncodableVersionedType, HashV1},
    version_bytes,
};
use anyhow::bail;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash, Serialize)]
pub struct LedgerHash(pub String);

impl LedgerHash {
    pub const PREFIX: &'static [&'static str] = &["jx", "jw", "jy", "jz"];
    pub const LEN: usize = 51;

    pub fn new<T>(hash: T) -> anyhow::Result<Self>
    where
        T: Into<String>,
    {
        let hash: String = hash.into();

        if Self::is_valid(&hash) {
            return Ok(Self(hash));
        }

        bail!("Invalid ledger hash: {hash}")
    }

    pub fn new_or_panic(hash: String) -> Self {
        Self::new(hash).expect("valid ledger hash")
    }

    pub fn from_hashv1(hashv1: HashV1) -> Self {
        let versioned: Base58EncodableVersionedType<{ version_bytes::LEDGER_HASH }, _> =
            hashv1.into();
        Self(versioned.to_base58_string().unwrap())
    }

    pub fn from_bytes(bytes: Vec<u8>) -> anyhow::Result<Self> {
        let hash = String::from_utf8(bytes)?;
        Self::new(hash)
    }

    pub fn from_bytes_or_panic(bytes: Vec<u8>) -> Self {
        Self::from_bytes(bytes).expect("ledger hash bytes")
    }

    pub fn is_valid(input: &str) -> bool {
        let prefix: String = input.chars().take(2).collect();
        input.len() == LedgerHash::LEN && LedgerHash::PREFIX.contains(&prefix.as_str())
    }
}

///////////
// serde //
///////////

impl<'de> Deserialize<'de> for LedgerHash {
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

impl std::str::FromStr for LedgerHash {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if Self::is_valid(s) {
            Ok(Self(s.to_string()))
        } else {
            bail!("Invalid ledger hash: {s}")
        }
    }
}

impl From<&str> for LedgerHash {
    fn from(value: &str) -> Self {
        Self::new_or_panic(value.to_string())
    }
}

/////////////
// default //
/////////////

impl std::default::Default for LedgerHash {
    fn default() -> Self {
        Self("jxDEFAULTDEFAULTDEFAULTDEFAULTDEFAULTDEFAULTDEFAULT".into())
    }
}

/////////////
// display //
/////////////

impl std::fmt::Display for LedgerHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

///////////////
// arbitrary //
///////////////

#[cfg(test)]
impl quickcheck::Arbitrary for LedgerHash {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let prefix = {
            let idx = u8::arbitrary(g) % Self::PREFIX.len() as u8;
            Self::PREFIX.get(idx as usize).expect("ledger hash prefix")
        };

        let mut chars = vec![];
        let alphabet: Vec<_> = ('a'..='z').chain('A'..='Z').chain('0'..='9').collect();

        for _ in 0..Self::LEN - 2 {
            let idx = usize::arbitrary(g) % alphabet.len();

            chars.push(alphabet.get(idx).cloned().unwrap());
        }

        Self(prefix.to_string() + &chars.iter().collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::LedgerHash;

    #[test]
    fn roundtrip() -> anyhow::Result<()> {
        let hash = LedgerHash::default();
        let hash_str = hash.to_string();

        // serialize
        let ser = serde_json::to_vec(&hash)?;

        // deserialize
        let res: LedgerHash = serde_json::from_slice(&ser)?;

        // roundtrip
        assert_eq!(hash, res);

        // same serialization as string
        assert_eq!(ser, serde_json::to_vec(&hash_str)?);

        Ok(())
    }
}
