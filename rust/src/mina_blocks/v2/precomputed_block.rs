use super::{protocol_state::ProtocolState, staged_ledger_diff::StagedLedgerDiff};
use crate::mina_blocks::common::from_str;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrecomputedBlock {
    pub version: u32,
    pub data: PrecomputedBlockData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrecomputedBlockData {
    /// Time the block is scheduled to be produced
    #[serde(deserialize_with = "from_str")]
    pub scheduled_time: u64,

    /// Summary of the current state
    pub protocol_state: ProtocolState,

    /// Collection of ledger updates
    pub staged_ledger_diff: StagedLedgerDiff,

    /// Protocol state proof
    #[serde(skip_deserializing)]
    pub protocol_state_proof: serde_json::Value,

    /// Delta transition chain proof
    #[serde(skip_deserializing)]
    pub delta_transition_chain_proof: serde_json::Value,
}

/// Parse
pub fn parse_file<P: AsRef<Path>>(path: P) -> anyhow::Result<PrecomputedBlock> {
    let contents = std::fs::read(path)?;
    Ok(serde_json::from_slice(&contents)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn deserialize() -> anyhow::Result<()> {
        let now = Instant::now();
        let path = "./tests/data/berkeley/sequential_blocks/berkeley-38-3NLMZGXbHqnGZ1pHo2D1Fyu6t29Qpqz8ExiRhrjkxFokzS1d2cLV.json";
        let block = parse_file(path)?;

        println!("Elapsed time: {:?}", now.elapsed());
        println!("{}", serde_json::to_string_pretty(&block)?);
        Ok(())
    }

    #[test]
    fn parse_berkeley_blocks() -> anyhow::Result<()> {
        glob::glob("./tests/data/berkeley/sequential_blocks/berkeley-*-*.json")?.for_each(|path| {
            if let Ok(ref path) = path {
                if let Err(e) = parse_file(path) {
                    panic!(
                        "Error parsing block {}: {}",
                        path.file_name().unwrap().to_str().unwrap(),
                        e
                    )
                }
            }
        });
        Ok(())
    }
}
