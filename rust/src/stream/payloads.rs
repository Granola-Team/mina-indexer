use sonic_rs::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct BlockAncestorPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
}

#[derive(Serialize, Deserialize)]
pub struct BerkeleyBlockPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
}

#[derive(Serialize, Deserialize)]
pub struct MainnetBlockPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
}

#[derive(Serialize, Deserialize)]
pub struct NewBlockAddedPayload {
    pub height: u64,
    pub state_hash: String,
}
