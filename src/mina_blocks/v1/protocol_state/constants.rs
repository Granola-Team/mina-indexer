use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Constants {
    pub k: String,
    pub slots_per_epoch: String,
    pub slots_per_sub_window: String,
    pub delta: String,
    pub genesis_state_timestamp: String,
}
