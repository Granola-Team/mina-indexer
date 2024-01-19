use crate::block::BlockHash;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PreviousStateHashBlock {
    pub protocol_state: ProtocolState,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProtocolState {
    pub previous_state_hash: String,
}

#[derive(PartialEq, Eq)]
pub struct PreviousStateHash(pub String);

impl PreviousStateHash {
    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        let PreviousStateHashBlock {
            protocol_state: ProtocolState {
                previous_state_hash,
            },
        } = serde_json::from_slice(&std::fs::read(path)?)?;
        Ok(Self(previous_state_hash))
    }
}

impl From<PreviousStateHashBlock> for PreviousStateHash {
    fn from(value: PreviousStateHashBlock) -> Self {
        Self(value.protocol_state.previous_state_hash)
    }
}

impl From<PreviousStateHashBlock> for BlockHash {
    fn from(value: PreviousStateHashBlock) -> Self {
        let p: PreviousStateHash = value.into();
        p.into()
    }
}

impl From<PreviousStateHash> for String {
    fn from(value: PreviousStateHash) -> Self {
        value.0
    }
}

impl From<PreviousStateHash> for BlockHash {
    fn from(value: PreviousStateHash) -> Self {
        value.0.into()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::block::precomputed::PrecomputedBlock;
    use std::path::PathBuf;

    #[test]
    fn check() -> anyhow::Result<()> {
        let path: PathBuf = "./tests/data/canonical_chain_discovery/contiguous/mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json".into();
        let PreviousStateHashBlock {
            protocol_state: ProtocolState {
                previous_state_hash,
            },
        } = serde_json::from_slice(&std::fs::read(&path)?)?;
        let pcb = PrecomputedBlock::parse_file(&path)?;
        assert_eq!(previous_state_hash, pcb.previous_state_hash().0);
        Ok(())
    }
}
