use sonic_rs::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct BerkeleyBlock {
    pub version: u32,
    pub data: Data,
}

impl BerkeleyBlock {
    pub fn get_previous_state_hash(&self) -> String {
        self.data.protocol_state.previous_state_hash.clone()
    }

    pub fn get_last_vrf_output(&self) -> String {
        self.data.protocol_state.body.consensus_state.last_vrf_output.clone()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Data {
    pub scheduled_time: String,
    pub protocol_state: ProtocolState,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProtocolState {
    pub previous_state_hash: String,
    pub body: Body,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Body {
    pub consensus_state: ConsensusState,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConsensusState {
    pub last_vrf_output: String,
}
