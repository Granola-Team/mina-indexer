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

impl VerificationKeyHash {
    pub const PREFIX: &str = "0x";
    pub const LEN: usize = 64;
}

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
        assert!(vk_hash.starts_with(Self::PREFIX));
        assert_eq!(vk_hash.len(), Self::LEN + Self::PREFIX.len());

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

///////////////
// arbitrary //
///////////////

#[cfg(test)]
impl quickcheck::Arbitrary for VerificationKeyData {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let len = u16::arbitrary(g);

        let mut chars = vec![];
        let alphabet: Vec<_> = ('a'..='z').chain('A'..='Z').chain('0'..='9').collect();

        for _ in 0..=len {
            let idx = usize::arbitrary(g) % alphabet.len();
            chars.push(alphabet.get(idx).cloned().unwrap());
        }

        Self(chars.iter().collect())
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for VerificationKeyHash {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let mut chars = vec![];
        let alphabet: Vec<_> = ('a'..='z').chain('A'..='Z').chain('0'..='9').collect();

        for _ in 0..VerificationKeyHash::LEN {
            let idx = usize::arbitrary(g) % alphabet.len();
            chars.push(alphabet.get(idx).cloned().unwrap());
        }

        Self(VerificationKeyHash::PREFIX.to_string() + &chars.iter().collect::<String>())
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for VerificationKey {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        Self {
            data: VerificationKeyData::arbitrary(g),
            hash: VerificationKeyHash::arbitrary(g),
        }
    }
}
