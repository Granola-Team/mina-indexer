//! Indexer internal V1 precomputed block representation

use super::MAINNET_GENESIS_TIMESTAMP;
use crate::{
    block::BlockHash,
    canonicity::Canonicity,
    chain::Network,
    mina_blocks::common::*,
    protocol::serialization_types::{
        protocol_state::{ProtocolState, ProtocolStateJson},
        staged_ledger_diff as mina_rs,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockFileV1 {
    #[serde(default = "mainnet_genesis_timestamp")]
    #[serde(deserialize_with = "from_str")]
    pub scheduled_time: u64,

    pub protocol_state: ProtocolStateJson,
    pub staged_ledger_diff: mina_rs::StagedLedgerDiffJson,
}

fn mainnet_genesis_timestamp() -> u64 {
    MAINNET_GENESIS_TIMESTAMP
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrecomputedBlockV1 {
    // metadata
    pub network: Network,
    pub state_hash: BlockHash,
    pub blockchain_length: u32,
    // from PCB
    pub scheduled_time: u64,
    pub protocol_state: ProtocolState,
    pub staged_ledger_diff: mina_rs::StagedLedgerDiff,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrecomputedBlockWithCanonicityV1 {
    pub canonicity: Option<Canonicity>,
    pub network: Network,
    pub state_hash: BlockHash,
    pub scheduled_time: u64,
    pub blockchain_length: u32,
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
