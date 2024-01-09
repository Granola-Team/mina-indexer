use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockWatcherEvent {
    WatchDir(PathBuf),
    SawBlock { state_hash: String, path: PathBuf },
}

impl std::fmt::Debug for BlockWatcherEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SawBlock { state_hash, .. } => write!(f, "block watcher: saw {state_hash}"),
            Self::WatchDir(path) => write!(f, "block watcher: Watch dir {}", path.display()),
        }
    }
}
