use crate::ledger::LedgerHash;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum StakingLedgerWatcherEvent {
    NewStakingLedger { epoch: u32, ledger_hash: LedgerHash },
}

impl std::fmt::Debug for StakingLedgerWatcherEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewStakingLedger { epoch, ledger_hash } => write!(
                f,
                "fs watcher saw staking ledger (epoch {}): {}",
                epoch, ledger_hash
            ),
        }
    }
}
