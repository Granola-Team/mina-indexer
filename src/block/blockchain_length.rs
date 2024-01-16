use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BlockchainLengthBlock {
    pub protocol_state: ProtocolState,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProtocolState {
    pub body: ProtocolStateBody,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProtocolStateBody {
    pub consensus_state: ConsensusState,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConsensusState {
    pub blockchain_length: String,
}

pub struct BlockchainLength(pub u32);

impl BlockchainLength {
    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        let BlockchainLengthBlock {
            protocol_state:
                ProtocolState {
                    body:
                        ProtocolStateBody {
                            consensus_state: ConsensusState { blockchain_length },
                        },
                },
        } = serde_json::from_slice(&std::fs::read(path)?)?;
        Ok(Self(blockchain_length.parse()?))
    }
}

impl From<BlockchainLength> for u32 {
    fn from(value: BlockchainLength) -> Self {
        value.0
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
        let BlockchainLengthBlock {
            protocol_state:
                ProtocolState {
                    body:
                        ProtocolStateBody {
                            consensus_state: ConsensusState { blockchain_length },
                        },
                },
        } = serde_json::from_slice(&std::fs::read(&path)?)?;
        let pcb = PrecomputedBlock::parse_file(&path)?;
        assert_eq!(blockchain_length.parse::<u32>()?, pcb.blockchain_length);
        Ok(())
    }
}
