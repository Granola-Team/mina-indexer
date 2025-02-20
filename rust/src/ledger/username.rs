use crate::{command::MEMO_LEN, constants::NAME_SERVICE_MEMO_PREFIX};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Username(pub String);

//////////
// impl //
//////////

impl Username {
    pub const MAX_LEN: usize = MEMO_LEN - NAME_SERVICE_MEMO_PREFIX.len();

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
