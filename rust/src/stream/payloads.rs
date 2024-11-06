use sonic_rs::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct BlockAncestorPayload {
    pub height: u64,
    pub state_hash: String,
    pub previous_state_hash: String,
}
