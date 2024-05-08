use crate::{chain_id::Network, ledger::LedgerHash};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum StakingLedgerWatcherEvent {
    NewStakingLedger {
        epoch: u32,
        network: Network,
        ledger_hash: LedgerHash,
    },
}

impl std::fmt::Debug for StakingLedgerWatcherEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewStakingLedger {
                epoch,
                network,
                ledger_hash,
            } => write!(
                f,
                "fs watcher saw {} staking ledger (epoch {}): {}",
                network, epoch, ledger_hash.0
            ),
        }
    }
}
