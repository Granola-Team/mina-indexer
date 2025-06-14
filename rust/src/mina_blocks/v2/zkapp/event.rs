//! Zkapp event representation

use crate::{base::state_hash::StateHash, command::TxnHash};
use serde::{Deserialize, Serialize};

/// 32 bytes
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappEvent(pub String);

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub struct ZkappEventWithMeta {
    pub event: ZkappEvent,
    pub txn_hash: TxnHash,
    pub state_hash: StateHash,
    pub block_height: u32,
}

//////////
// impl //
//////////

impl ZkappEvent {
    pub const PREFIX: &'static str = "0x";

    // 32 bytes = 64 hex + 2 prefix chars
    pub const LEN: usize = 66;
}

/////////////////
// conversions //
/////////////////

impl<T> From<T> for ZkappEvent
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        let event = value.into();

        // 32 bytes = 64 hex + 2 prefix chars
        assert!(event.starts_with(Self::PREFIX));
        assert_eq!(event.len(), Self::LEN);

        Self(event)
    }
}

/////////////
// default //
/////////////

impl std::default::Default for ZkappEvent {
    fn default() -> Self {
        Self(Self::PREFIX.to_string() + &"0".repeat(Self::LEN - 2))
    }
}

/////////////
// display //
/////////////

impl std::fmt::Display for ZkappEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for ZkappEvent {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let mut bytes = [0u8; 32];

        for byte in bytes.iter_mut() {
            *byte = u8::arbitrary(g);
        }

        Self(format!("0x{}", hex::encode(bytes)))
    }
}
