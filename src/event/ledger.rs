use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LedgerEvent {
    NewLedger { hash: String, path: PathBuf },
    WatchDir(PathBuf),
}
