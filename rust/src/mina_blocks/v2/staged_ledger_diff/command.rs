use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    UserCommand(UserCommand),
    ZkappCommand(ZkappCommand),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCommand {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkappCommand {}
