//! Indexer scheduled time type

use crate::constants::{HARDFORK_GENESIS_TIMESTAMP, MAINNET_GENESIS_TIMESTAMP};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd)]
pub struct ScheduledTime(pub u64);

//////////
// impl //
//////////

impl ScheduledTime {
    pub fn mainnet_genesis_timestamp() -> Self {
        Self(MAINNET_GENESIS_TIMESTAMP)
    }

    pub fn hardfork_genesis_timestamp() -> Self {
        Self(HARDFORK_GENESIS_TIMESTAMP)
    }
}

/////////////////
// conversions //
/////////////////

impl FromStr for ScheduledTime {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let time = s.parse()?;
        Ok(Self(time))
    }
}

///////////
// serde //
///////////

impl Serialize for ScheduledTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        crate::utility::serde::to_str(self, serializer)
    }
}

impl<'de> Deserialize<'de> for ScheduledTime {
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

impl Display for ScheduledTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::ScheduledTime;

    #[test]
    fn roundtrip() -> anyhow::Result<()> {
        let time = ScheduledTime::default();
        let time_str = time.to_string();

        // serialize
        let ser = serde_json::to_vec(&time)?;

        // deserialize
        let res: ScheduledTime = serde_json::from_slice(&ser)?;

        // roundtrip
        assert_eq!(time, res);

        // same serialization as string
        assert_eq!(ser, serde_json::to_vec(&time_str)?);

        Ok(())
    }
}
