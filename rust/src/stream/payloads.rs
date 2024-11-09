use sonic_rs::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct BlockAncestorPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
    pub last_vrf_output: String,
}

#[derive(Serialize, Deserialize)]
pub struct BerkeleyBlockPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
    pub last_vrf_output: String,
}

#[derive(Serialize, Deserialize)]
pub struct MainnetBlockPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
    pub last_vrf_output: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NewBlockAddedPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
    pub last_vrf_output: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct BlockCanonicityUpdatePayload {
    pub height: u64,
    pub state_hash: String,
    pub canonical: bool,
}
