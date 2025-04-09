//! Voting for representation

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct VotingFor(pub String);

impl<T> From<T> for VotingFor
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

///////////
// serde //
///////////

impl<'de> Deserialize<'de> for VotingFor {
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

impl std::str::FromStr for VotingFor {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

/////////////
// default //
/////////////

impl std::default::Default for VotingFor {
    fn default() -> Self {
        Self("3NK2tkzqqK5spR2sZ7tujjqPksL45M3UUrcA4WhCkeiPtnugyE2x".to_string())
    }
}
