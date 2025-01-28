//! Indexer internal V1 precomputed block representation

use crate::{
    base::{
        blockchain_length::BlockchainLength, scheduled_time::ScheduledTime, state_hash::StateHash,
    },
    canonicity::Canonicity,
    chain::Network,
    protocol::serialization_types::{
        protocol_state::{ProtocolState, ProtocolStateJson},
        staged_ledger_diff as mina_rs,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockFileV1 {
    #[serde(default = "ScheduledTime::mainnet_genesis_timestamp")]
    pub scheduled_time: ScheduledTime,

    pub protocol_state: ProtocolStateJson,
    pub staged_ledger_diff: mina_rs::StagedLedgerDiffJson,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrecomputedBlockV1 {
    // metadata
    pub network: Network,
    pub state_hash: StateHash,
    pub blockchain_length: BlockchainLength,
    // from PCB
    pub scheduled_time: ScheduledTime,
    pub protocol_state: ProtocolState,
    pub staged_ledger_diff: mina_rs::StagedLedgerDiff,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrecomputedBlockWithCanonicityV1 {
    pub canonicity: Option<Canonicity>,
    pub network: Network,
    pub state_hash: StateHash,
    pub scheduled_time: ScheduledTime,
    pub blockchain_length: BlockchainLength,
    pub protocol_state: ProtocolState,
    pub staged_ledger_diff: mina_rs::StagedLedgerDiff,
}

#[cfg(test)]
mod tests {
    use crate::block::{genesis::GenesisBlock, precomputed::PrecomputedBlock};

    #[test]
    fn serde_roundtrip() -> anyhow::Result<()> {
        // v1 block
        let before = GenesisBlock::new_v1()?.to_precomputed();

        let bytes = serde_json::to_vec(&before)?;
        let after = serde_json::from_slice::<PrecomputedBlock>(&bytes)?;

        assert_eq!(before, after);
        Ok(())
    }
}
