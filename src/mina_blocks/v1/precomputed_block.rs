use super::{
    common::from_str, protocol_state::ProtocolState, staged_ledger_diff::StagedLedgerDiff,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrecomputedBlock {
    /// Time the block is scheduled to be produced
    #[serde(deserialize_with = "from_str")]
    pub scheduled_time: u64,
    /// Summary of the current state
    pub protocol_state: ProtocolState,
    /// Collection of ledger updates
    pub staged_ledger_diff: StagedLedgerDiff,
    #[serde(skip_deserializing)]
    pub protocol_state_proof: serde_json::Value,
    #[serde(skip_deserializing)]
    pub delta_transition_chain_proof: serde_json::Value,
}

pub fn parse_file<P: AsRef<Path>>(path: P) -> anyhow::Result<PrecomputedBlock> {
    let contents = std::fs::read(path)?;
    let block = serde_json::from_slice(&contents)?;
    Ok(block)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn deserialize() -> anyhow::Result<()> {
        let path = "./tests/data/non_sequential_blocks/mainnet-40702-3NLkEG6S6Ra8Z1i5U5MPSNWV13hzQV8pYx1xBaeLDFN4EJhSuksw.json";
        let now = Instant::now();
        let block = parse_file(path)?;
        println!("{}", serde_json::to_string_pretty(&block)?);
        println!("Elapsed time: {:?}", now.elapsed());
        Ok(())
    }
}
