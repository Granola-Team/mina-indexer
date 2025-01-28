use crate::{
    base::scheduled_time::ScheduledTime,
    ledger::{token::TokenId, LedgerHash},
};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockchainState {
    pub timestamp: ScheduledTime,
    pub snarked_next_available_token: TokenId,
    pub snarked_ledger_hash: LedgerHash,
    pub genesis_ledger_hash: LedgerHash,
    pub staged_ledger_hash: StagedLedgerHash,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StagedLedgerHash {
    pub non_snark: NonSnark,
    pub pending_coinbase_hash: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NonSnark {
    pub ledger_hash: LedgerHash,
    pub aux_hash: String,
    pub pending_coinbase_aux: String,
}
