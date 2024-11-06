use sonic_rs::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct MainnetBlock {
    pub scheduled_time: String,
    pub protocol_state: ProtocolState,
}

impl MainnetBlock {
    pub fn get_previous_state_hash(&self) -> String {
        self.protocol_state.previous_state_hash.clone()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProtocolState {
    pub previous_state_hash: String,
}
