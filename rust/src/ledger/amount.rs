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

impl ToString for Amount {
    fn to_string(&self) -> String {
        nanomina_to_mina(self.0)
    }
}

impl Add<Amount> for Amount {
    type Output = Amount;

    fn add(self, rhs: Amount) -> Self::Output {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl Add<u64> for Amount {
    type Output = Amount;

    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0.saturating_add(rhs))
    }
}

impl Add<i64> for Amount {
    type Output = Amount;

    fn add(self, rhs: i64) -> Self::Output {
        let abs = rhs.unsigned_abs();
        if rhs > 0 {
            Self(self.0.saturating_add(abs))
        } else {
            Self(self.0.saturating_sub(abs))
        }
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
        Self(self.0.saturating_sub(rhs))
    }
}

impl From<u64> for Amount {
    fn from(value: u64) -> Self {
        Amount(value)
    }
}
