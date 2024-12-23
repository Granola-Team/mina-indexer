use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};

#[derive(
    PartialEq, Eq, Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, Ord, Hash,
)]
pub struct Nonce(pub u32);

// operations

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

// conversions

impl From<u32> for Nonce {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<String> for Nonce {
    fn from(value: String) -> Self {
        Self(value.parse::<u32>().expect("nonce is u32"))
    }
}

impl From<Nonce> for serde_json::value::Number {
    fn from(value: Nonce) -> Self {
        Self::from(value.0)
    }
}

// display

impl std::fmt::Display for Nonce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
