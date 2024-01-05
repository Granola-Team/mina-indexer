use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockEvent {
    WatchDir(PathBuf),
    SawBlock(PathBuf),
}
