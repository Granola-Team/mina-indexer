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
    pub user_command_count: usize,
    pub snark_work_count: usize,
    pub timestamp: u64,
    pub coinbase_receiver: String,
    pub coinbase_reward_nanomina: u64,
    pub global_slot_since_genesis: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NewBlockPayload {
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

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct GenesisBlockPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
    pub last_vrf_output: String,
    pub unix_timestamp: u64,
}

impl Default for GenesisBlockPayload {
    fn default() -> Self {
        Self::new()
    }
}

impl GenesisBlockPayload {
    pub fn new() -> Self {
        Self {
            height: 1,
            state_hash: "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ".to_string(),
            previous_state_hash: "3NLoKn22eMnyQ7rxh5pxB6vBA3XhSAhhrf7akdqS6HbAKD14Dh1d".to_string(),
            last_vrf_output: "NfThG1r1GxQuhaGLSJWGxcpv24SudtXG4etB0TnGqwg=".to_string(),
            unix_timestamp: 1615939200000,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct BlockSummaryPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
    pub user_command_count: usize,
    pub snark_work_count: usize,
    pub timestamp: u64,
    pub coinbase_receiver: String,
    pub coinbase_reward_nanomina: u64,
    pub global_slot_since_genesis: u64,
    pub is_berkeley_block: bool,
}
