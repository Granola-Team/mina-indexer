use crate::block::BlockHash;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

use std::fs::File;
use std::io::{BufReader, Read};

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
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Read the entire file into a buffer (assuming it's large, but you only need the start)
        let mut buffer = String::new();
        reader.read_to_string(&mut buffer)?;

        // Locate "protocol_state" within the JSON
        if let Some(start_pos) = buffer.find("\"protocol_state\":") {
            if let Some(brace_pos) = buffer[start_pos..].find('{') {
                let actual_start = start_pos + brace_pos;

                // Extract a fixed length portion of the buffer after the "protocol_state"
                let slice = &buffer[actual_start..actual_start + 77];
                let slice_with_brace = format!("{}{}", slice, "}"); // Add the closing brace manually

                // Deserialize just this portion
                let v: Value = serde_json::from_str(&slice_with_brace)?;

                // Extract "previous_state_hash"
                if let Some(previous_state_hash) = v.get("previous_state_hash") {
                    // Convert to string and return
                    if let Some(hash_str) = previous_state_hash.as_str() {
                        return Ok(Self(hash_str.to_string()));
                    }
                }
            }
        }

        // Return an error if the previous_state_hash was not found
        Err(anyhow::anyhow!(
            "Failed to find previous_state_hash in the file"
        ))
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

    #[test]
    fn previous_state_hash_deserializer_test() -> anyhow::Result<()> {
        let paths: Vec<PathBuf> =
            glob::glob("./tests/data/canonical_chain_discovery/contiguous/*.json")?
                .filter_map(|x| x.ok())
                .collect();

        for path in paths {
            let previous_state_hash = PreviousStateHash::from_path(&path)?.0;
            let block = PrecomputedBlock::parse_file(&path, PcbVersion::V1)?;
            assert_eq!(previous_state_hash, block.previous_state_hash().0);
        }
        Ok(())
    }
}
