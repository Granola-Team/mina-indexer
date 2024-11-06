use sonic_rs::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct MainnetBlock {
    pub scheduled_time: String,
    pub protocol_state: ProtocolState,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProtocolState {
    pub previous_state_hash: String,
}
