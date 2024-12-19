use crate::protocol::serialization_types::{
    common::{Base58EncodableVersionedType, HashV1},
    version_bytes,
};
use anyhow::bail;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LedgerHash(pub String);

impl LedgerHash {
    pub const PREFIX: &'static [&'static str] = &["jx", "jw", "jy", "jz"];
    pub const LEN: usize = 51;

    pub fn new(hash: String) -> anyhow::Result<Self> {
        if is_valid_ledger_hash(&hash) {
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
}

impl std::str::FromStr for LedgerHash {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if is_valid_ledger_hash(s) {
            Ok(Self(s.to_string()))
        } else {
            bail!("Invalid ledger hash: {s}")
        }
    }
}

pub fn is_valid_ledger_hash(input: &str) -> bool {
    let prefix: String = input.chars().take(2).collect();
    input.len() == LedgerHash::LEN && LedgerHash::PREFIX.contains(&prefix.as_str())
}

impl std::default::Default for LedgerHash {
    fn default() -> Self {
        Self("jxDEFAULTDEFAULTDEFAULTDEFAULTDEFAULTDEFAULTDEFAULT".into())
    }
}

impl std::fmt::Display for LedgerHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
