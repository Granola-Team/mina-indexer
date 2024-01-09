use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum LedgerWatcherEvent {
    NewLedger { hash: String, path: PathBuf },
    WatchDir(PathBuf),
}

impl std::fmt::Debug for LedgerWatcherEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewLedger { hash, .. } => write!(f, "ledger: new ({hash})"),
            Self::WatchDir(path) => write!(f, "ledger: watch dir ({})", path.display()),
        }
    }
}
