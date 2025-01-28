//! Indexer internal V2 precomputed block representation

use crate::{
    base::{
        blockchain_length::BlockchainLength, scheduled_time::ScheduledTime, state_hash::StateHash,
    },
    canonicity::Canonicity,
    chain::Network,
    mina_blocks::v2,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockFileV2 {
    pub version: u32,
    pub data: BlockFileDataV2,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockFileDataV2 {
    #[serde(default = "ScheduledTime::hardfork_genesis_timestamp")]
    pub scheduled_time: ScheduledTime,

    pub protocol_state: v2::protocol_state::ProtocolState,
    pub staged_ledger_diff: v2::staged_ledger_diff::StagedLedgerDiff,

    // new post-hardfork data
    pub tokens_used: Vec<v2::TokenUsed>,
    pub accounts_accessed: Vec<(u64, v2::AccountAccessed)>,
    pub accounts_created: Vec<v2::AccountCreated>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrecomputedBlockV2 {
    // metadata
    pub network: Network,
    pub state_hash: StateHash,
    pub blockchain_length: BlockchainLength,
    // from PCB
    pub scheduled_time: ScheduledTime,
    pub protocol_state: v2::protocol_state::ProtocolState,
    pub staged_ledger_diff: v2::staged_ledger_diff::StagedLedgerDiff,
    // new post-hardfork data
    pub tokens_used: Vec<v2::TokenUsed>,
    pub accounts_accessed: Vec<(u64, v2::AccountAccessed)>,
    pub accounts_created: Vec<v2::AccountCreated>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrecomputedBlockWithCanonicityV2 {
    pub canonicity: Option<Canonicity>,
    pub network: Network,
    pub state_hash: StateHash,
    pub scheduled_time: ScheduledTime,
    pub blockchain_length: BlockchainLength,
    pub protocol_state: v2::protocol_state::ProtocolState,
    pub staged_ledger_diff: v2::staged_ledger_diff::StagedLedgerDiff,
}

#[cfg(test)]
mod tests {
    use crate::block::{genesis::GenesisBlock, precomputed::PrecomputedBlock};

    #[test]
    fn serde_roundtrip() -> anyhow::Result<()> {
        // v2 block
        let before = GenesisBlock::new_v2()?.to_precomputed();

        let bytes = serde_json::to_vec(&before)?;
        let after = serde_json::from_slice::<PrecomputedBlock>(&bytes)?;

        assert_eq!(before, after);
        Ok(())
    }
}
