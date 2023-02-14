use serde_json::Value;

pub mod reader;

pub struct BlockLog {
    pub state_hash: String,
    pub json: Value,
}