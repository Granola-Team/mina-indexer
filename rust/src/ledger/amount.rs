use crate::{constants::MINA_SCALE, utility::functions::nanomina_to_mina};
use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};

#[derive(
    Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default, Hash, Serialize, Deserialize,
)]
pub struct Amount(pub u64);

impl Amount {
    pub fn new(amount: u64) -> Self {
        Self(amount * MINA_SCALE)
    }
}

// operations

impl Add<Amount> for Amount {
    type Output = Amount;

    fn add(self, rhs: Amount) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Add<u64> for Amount {
    type Output = Amount;

    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Add<i64> for Amount {
    type Output = Amount;

    fn add(self, rhs: i64) -> Self::Output {
        Self(self.0 + rhs as u64)
    }
}

impl Sub<Amount> for Amount {
    type Output = Amount;

    fn sub(self, rhs: Amount) -> Self::Output {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl Sub<u64> for Amount {
    type Output = Amount;

    fn sub(self, rhs: u64) -> Self::Output {
        self - Self(rhs)
    }
}

// converisons

impl From<u64> for Amount {
    fn from(value: u64) -> Self {
        Amount(value)
    }
}

// display

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", nanomina_to_mina(self.0))
    }
}
