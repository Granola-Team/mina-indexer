use crate::mina_blocks::v1::common::from_str;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockchainState {
    #[serde(deserialize_with = "from_str")]
    pub timestamp: u64,
    #[serde(deserialize_with = "from_str")]
    pub snarked_next_available_token: u64,
    pub staged_ledger_hash: StagedLedgerHash,
    pub snarked_ledger_hash: String,
    pub genesis_ledger_hash: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StagedLedgerHash {
    pub non_snark: NonSnark,
    pub pending_coinbase_hash: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NonSnark {
    pub ledger_hash: String,
    pub aux_hash: String,
    pub pending_coinbase_aux: String,
}
