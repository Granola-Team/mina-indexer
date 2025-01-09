use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VerificationKey {
    pub data: VerificationKeyData,
    pub hash: VerificationKeyHash,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VerificationKeyData(pub String);

/// 32 bytes
#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VerificationKeyHash(pub String);

/////////////////
// conversions //
/////////////////

impl<T> From<T> for VerificationKeyHash
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        let vk_hash = value.into();

        // 32 bytes = 64 hex + 2 prefix chars
        assert!(vk_hash.starts_with("0x"));
        assert_eq!(vk_hash.len(), 66);

        Self(vk_hash)
    }
}

impl<T> From<T> for VerificationKeyData
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}
