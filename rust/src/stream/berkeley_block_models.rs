use sonic_rs::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct BerkeleyBlock {
    pub version: u32,
    pub data: Data,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Data {
    pub scheduled_time: String,
    pub protocol_state: ProtocolState,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProtocolState {
    pub previous_state_hash: String,
}
