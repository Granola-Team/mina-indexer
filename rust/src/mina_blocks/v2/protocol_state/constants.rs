use super::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Constants {
    pub k: Numeric<u32>,
    pub slots_per_epoch: Numeric<u32>,
    pub slots_per_sub_window: Numeric<u32>,
    pub delta: Numeric<u32>,
    pub genesis_state_timestamp: ScheduledTime,
}
