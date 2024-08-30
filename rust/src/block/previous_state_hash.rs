use crate::block::BlockHash;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

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
        let reader = BufReader::new(file);

        // Create a buffer with a capacity of 1Kb
        let mut buffer = String::with_capacity(1000);

        // Limit the reader to read only the first 300 bytes
        reader.take(1000).read_to_string(&mut buffer)?;

        // Locate "previous_state_hash" within the buffer
        let prev_state_hash_key = "\"previous_state_hash\"";
        if let Some(hash_pos) = buffer.find(prev_state_hash_key) {
            let hash_start = hash_pos + prev_state_hash_key.len();

            // Find the first quote after the colon
            if let Some(quote_start) = buffer[hash_start..].find('"') {
                let actual_start = hash_start + quote_start + 1; // Move to the start of the hash

                // Use a fixed length of 52 characters for the hash
                let hash_end = actual_start + 52;

                if hash_end <= buffer.len() {
                    let previous_state_hash = &buffer[actual_start..hash_end];
                    return Ok(Self(previous_state_hash.to_string()));
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
mod previous_state_hash_tests {
    use super::*;
    use crate::block::precomputed::{PcbVersion, PrecomputedBlock};
    use std::{fs::write, path::PathBuf};

    #[test]
    fn previous_state_hash_deserializer_test() -> anyhow::Result<()> {
        // Test cases with different formats
        let test_cases = [
            (
                r#"{"protocol_state": {"previous_state_hash": "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw""#,
                "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw",
            ),
            (
                r#"{
                        "protocol_state": {
                            "previous_state_hash": "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw"
                     "#,
                "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw",
            ),
            (
                r#"{"protocol_state":
                        {    "previous_state_hash"    :    "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw""#,
                "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw",
            ),
            (
                r#"{"protocol_state": {
                            "previous_state_hash":
                            "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw"
                        "#,
                "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw",
            ),
        ];

        for (i, (json_content, expected_hash)) in test_cases.iter().enumerate() {
            let test_path = format!(
                "./tests/data/canonical_chain_discovery/contiguous/test_case_{}.json",
                i
            );
            write(&test_path, json_content)?;

            let previous_state_hash = PreviousStateHash::from_path(Path::new(&test_path))?.0;
            assert_eq!(previous_state_hash, *expected_hash);

            // Cleanup the test file
            std::fs::remove_file(test_path)?;
        }

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
