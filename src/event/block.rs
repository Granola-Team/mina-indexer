use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockEvent {
    WatchDir(PathBuf),
    SawBlock(PathBuf),
}

impl std::fmt::Debug for BlockEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SawBlock(path) => write!(f, "block: saw ({})", path.display()),
            Self::WatchDir(path) => write!(f, "block: Watch dir ({})", path.display()),
        }
    }
}
