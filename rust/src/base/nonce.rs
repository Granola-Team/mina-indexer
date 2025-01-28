//! Indexer nonce type

use serde::{Deserialize, Serialize};
use std::{
    ops::{Add, Sub},
    str::FromStr,
};

#[derive(PartialEq, Eq, Debug, Copy, Clone, Default, PartialOrd, Ord, Hash)]
pub struct Nonce(pub u32);

////////////////
// operations //
////////////////

impl Add<u32> for Nonce {
    type Output = Nonce;

    fn add(self, other: u32) -> Nonce {
        Self(self.0.saturating_add(other))
    }
}

impl Sub<u32> for Nonce {
    type Output = Nonce;

    fn sub(self, other: u32) -> Nonce {
        Self(self.0.saturating_sub(other))
    }
}

impl Add<i32> for Nonce {
    type Output = Nonce;

    fn add(self, other: i32) -> Nonce {
        let abs = other.unsigned_abs();
        if other > 0 {
            Self(self.0.saturating_add(abs))
        } else {
            Self(self.0.saturating_sub(abs))
        }
    }
}

///////////
// serde //
///////////

impl Serialize for Nonce {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        crate::utility::serde::to_str(self, serializer)
    }
}

impl<'de> Deserialize<'de> for Nonce {
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

impl FromStr for Nonce {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

impl From<String> for Nonce {
    fn from(value: String) -> Self {
        Self::from_str(&value).expect("nonce")
    }
}

impl From<u32> for Nonce {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

/////////////
// display //
/////////////

impl std::fmt::Display for Nonce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Nonce;

    #[test]
    fn roundtrip() -> anyhow::Result<()> {
        let nonce = Nonce::default();
        let nonce_str = nonce.to_string();

        // serialize
        let ser = serde_json::to_vec(&nonce)?;

        // deserialize
        let res: Nonce = serde_json::from_slice(&ser)?;

        // roundtrip
        assert_eq!(nonce, res);

        // same serialization as string
        assert_eq!(ser, serde_json::to_vec(&nonce_str)?);

        Ok(())
    }
}
