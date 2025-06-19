//! Indexer amount type

use crate::{
    constants::{MAINNET_ACCOUNT_CREATION_FEE, MINA_SCALE, MINA_SCALE_DEC},
    utility::functions::nanomina_to_mina,
};
use anyhow::anyhow;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Sub, SubAssign};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default, Hash)]
pub struct Amount(pub u64);

//////////
// impl //
//////////

impl Amount {
    pub fn new(amount: u64) -> Self {
        Self(amount * MINA_SCALE)
    }

    pub fn to_f64(&self) -> f64 {
        use rust_decimal::prelude::ToPrimitive;

        let mut decimal = Decimal::from(self.0);
        decimal.set_scale(9).ok();

        decimal.to_f64().unwrap()
    }

    pub fn deduct_mina_account_creation_fee(&self, is_mina_creation_fee_paid: bool) -> u64 {
        if is_mina_creation_fee_paid {
            self.0
        } else {
            self.0 - MAINNET_ACCOUNT_CREATION_FEE.0
        }
    }
}

////////////////
// operations //
////////////////

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
        let abs = rhs.unsigned_abs();

        if rhs < 0 {
            Self(self.0 - abs)
        } else {
            Self(self.0 + abs)
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
        Self(self.0 - rhs)
    }
}

impl AddAssign<Amount> for Amount {
    fn add_assign(&mut self, rhs: Amount) {
        *self += rhs.0
    }
}

impl AddAssign<u64> for Amount {
    fn add_assign(&mut self, rhs: u64) {
        *self = Self(self.0 + rhs)
    }
}

impl AddAssign<i64> for Amount {
    fn add_assign(&mut self, rhs: i64) {
        let abs = rhs.unsigned_abs();

        if rhs < 0 {
            *self -= abs;
        } else {
            *self += abs;
        }
    }
}

impl SubAssign<u64> for Amount {
    fn sub_assign(&mut self, rhs: u64) {
        *self = Self(self.0 - rhs)
    }
}

impl SubAssign<i64> for Amount {
    fn sub_assign(&mut self, rhs: i64) {
        let abs = rhs.unsigned_abs();

        if rhs < 0 {
            *self += abs;
        } else {
            *self -= abs;
        }
    }
}

/////////////////
// converisons //
/////////////////

impl From<u64> for Amount {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl std::str::FromStr for Amount {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use rust_decimal::prelude::ToPrimitive;

        s.parse::<Decimal>()
            .map(|amt| Self((amt * MINA_SCALE_DEC).to_u64().expect("amount")))
            .map_err(|e| anyhow!("{e}"))
    }
}

///////////
// serde //
///////////

impl Serialize for Amount {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        crate::utility::serde::to_nanomina_str(&self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for Amount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        crate::utility::serde::from_nanomina_str(deserializer).map(Self)
    }
}

/////////////
// display //
/////////////

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", nanomina_to_mina(self.0))
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for Amount {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let amt = u64::arbitrary(g);
        amt.into()
    }
}

#[cfg(test)]
mod tests {
    use super::Amount;
    use std::str::FromStr;

    #[test]
    fn roundtrip() -> anyhow::Result<()> {
        let amt = Amount::default();

        // serialize
        let ser = serde_json::to_vec(&amt)?;

        // deserialize
        let res: Amount = serde_json::from_slice(&ser)?;

        // roundtrip
        assert_eq!(amt, res);

        // same serialization as string
        let amt = Amount::from_str("3.141592")?;
        let amt_str = amt.to_string();

        assert_eq!(serde_json::to_vec(&amt)?, serde_json::to_vec(&amt_str)?);

        Ok(())
    }
}
