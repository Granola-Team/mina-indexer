use super::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Constants {
    #[serde(serialize_with = "to_str")]
    #[serde(deserialize_with = "from_str")]
    pub k: u32,

    #[serde(serialize_with = "to_str")]
    #[serde(deserialize_with = "from_str")]
    pub slots_per_epoch: u32,

    #[serde(serialize_with = "to_str")]
    #[serde(deserialize_with = "from_str")]
    pub slots_per_sub_window: u32,

    #[serde(serialize_with = "to_str")]
    #[serde(deserialize_with = "from_str")]
    pub delta: u32,

    #[serde(serialize_with = "to_str")]
    #[serde(deserialize_with = "from_str")]
    pub genesis_state_timestamp: u64,
}
