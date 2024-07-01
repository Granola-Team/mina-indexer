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
        let bytes = &std::fs::read(path)?;
        let PreviousStateHashBlock {
            protocol_state: ProtocolState {
                previous_state_hash,
            },
        } = serde_json::from_slice(bytes)?;
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
    use crate::block::precomputed::{PcbVersion, PrecomputedBlock};
    use std::path::PathBuf;

    #[tokio::test]
    async fn previous_state_hash_deserializer_test() -> anyhow::Result<()> {
        let paths: Vec<PathBuf> =
            glob::glob("./tests/data/canonical_chain_discovery/contiguous/*.json")?
                .filter_map(|x| x.ok())
                .collect();

        for path in paths {
            let previous_state_hash = PreviousStateHash::from_path(&path)?.0;
            let block = PrecomputedBlock::parse_file(&path, PcbVersion::V1).await?;
            assert_eq!(previous_state_hash, block.previous_state_hash().0);
        }
        Ok(())
    }
}
