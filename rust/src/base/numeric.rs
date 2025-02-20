//! Indexer numeric type

use serde::{Deserialize, Serialize};
use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Numeric<T>(pub T)
where
    T: Default + Copy + Clone + PartialEq + Eq + PartialOrd + Ord;

/////////////////
// conversions //
/////////////////

impl<T> FromStr for Numeric<T>
where
    T: Default + Copy + Clone + PartialEq + Eq + PartialOrd + Ord + FromStr,
    <T as FromStr>::Err: Debug,
{
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let num = s
            .parse::<T>()
            .map_err(|e| anyhow::anyhow!("Failed to parse number '{}' : {:?}", s, e))?;
        Ok(Self(num))
    }
}

impl<T> From<T> for Numeric<T>
where
    T: Default + Copy + Clone + PartialEq + Eq + PartialOrd + Ord,
{
    fn from(value: T) -> Self {
        Self(value)
    }
}

///////////
// serde //
///////////

impl<T> Serialize for Numeric<T>
where
    T: Default + Copy + Clone + PartialEq + Eq + PartialOrd + Ord + Display,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        crate::utility::serde::to_str(self, serializer)
    }
}

impl<'de, T> Deserialize<'de> for Numeric<T>
where
    T: Default + Copy + Clone + PartialEq + Eq + PartialOrd + Ord + FromStr,
    <T as FromStr>::Err: Debug,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        crate::utility::serde::from_str(deserializer)
    }
}

/////////////
// display //
/////////////

impl<T> Display for Numeric<T>
where
    T: Default + Copy + Clone + PartialEq + Eq + PartialOrd + Ord + Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Numeric;

    #[test]
    fn roundtrip() -> anyhow::Result<()> {
        /////////
        // u32 //
        /////////

        let num = <Numeric<u32>>::default();
        let num_str = num.to_string();

        // serialize
        let ser = serde_json::to_vec(&num)?;

        // deserialize
        let res: Numeric<u32> = serde_json::from_slice(&ser)?;

        // roundtrip
        assert_eq!(num, res);

        // same serialization as string
        assert_eq!(ser, serde_json::to_vec(&num_str)?);

        /////////
        // u64 //
        /////////

        let num = <Numeric<u64>>::default();
        let num_str = num.to_string();

        // serialize
        let ser = serde_json::to_vec(&num)?;

        // deserialize
        let res: Numeric<u64> = serde_json::from_slice(&ser)?;

        // roundtrip
        assert_eq!(num, res);

        // same serialization as string
        assert_eq!(ser, serde_json::to_vec(&num_str)?);

        Ok(())
    }
}
