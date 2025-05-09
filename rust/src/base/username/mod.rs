//! Username representation

pub mod off_chain;

use crate::{command::MEMO_LEN, constants::NAME_SERVICE_MEMO_PREFIX};
use anyhow::{anyhow, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Username(pub String);

//////////
// impl //
//////////

impl Username {
    pub const MAX_LEN: usize = MEMO_LEN - NAME_SERVICE_MEMO_PREFIX.len();

    fn is_valid(username: &str) -> bool {
        username.len() <= Self::MAX_LEN
    }

    pub fn new(username: impl Into<String>) -> anyhow::Result<Self> {
        let username: String = username.into();

        if Self::is_valid(&username) {
            Ok(Self(username))
        } else {
            bail!("Invalid username: {}", username)
        }
    }

    pub fn from_bytes(bytes: Vec<u8>) -> anyhow::Result<Self> {
        String::from_utf8(bytes)
            .map_err(|e| anyhow!("Error deserializing username: {e}"))
            .map(Self)
    }
}

/////////////////
// conversions //
/////////////////

impl From<&str> for Username {
    fn from(value: &str) -> Self {
        let username = value.to_string();
        Self(username)
    }
}

///////////////////
// display/debug //
///////////////////

impl std::default::Default for Username {
    fn default() -> Self {
        Self("Unknown".to_string())
    }
}

impl std::fmt::Display for Username {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

///////////////
// arbitrary //
///////////////

#[cfg(test)]
impl quickcheck::Arbitrary for Username {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let mut chars = vec![];
        let alphabet: Vec<_> = ('a'..='z').chain('A'..='Z').chain('0'..='9').collect();

        for _ in 0..MEMO_LEN - NAME_SERVICE_MEMO_PREFIX.len() {
            let idx = usize::arbitrary(g) % alphabet.len();

            chars.push(alphabet.get(idx).cloned().unwrap());
        }

        Self(chars.iter().collect())
    }
}

#[cfg(test)]
impl Username {
    pub fn arbitrary_not(g: &mut quickcheck::Gen, username: &Self) -> Self {
        use quickcheck::Arbitrary;

        let mut name = Self::arbitrary(g);
        while name == *username {
            name = Self::arbitrary(g);
        }

        name
    }
}
