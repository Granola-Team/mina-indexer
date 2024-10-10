use anyhow::bail;
use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

pub struct BlockchainLength(u32);

impl BlockchainLength {
    pub fn from_path(path: &Path) -> anyhow::Result<u32> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut buffer = String::with_capacity(1000);

        // Limit the reader to read only the first 1000 bytes
        reader.take(1000).read_to_string(&mut buffer)?;

        // Locate "blockchain_length" within the buffer
        let blockchain_length_key = "\"blockchain_length\"";
        if let Some(length_pos) = buffer.find(blockchain_length_key) {
            let length_start = length_pos + blockchain_length_key.len();

            // Find the first quote after the colon
            if let Some(quote_start) = buffer[length_start..].find('"') {
                let start = length_start + quote_start + 1;
                if let Some(end) = buffer[start..].find('"') {
                    if start + end <= buffer.len() {
                        let blockchain_length = &buffer[start..][..end];
                        return Ok(blockchain_length.parse()?);
                    }
                }
            }
        }
        bail!("Failed to find blockchain_length in the file")
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
    use crate::block::precomputed::{PcbVersion, PrecomputedBlock};
    use std::fs::write;
    use tempfile::TempDir;

    #[test]
    fn blockchain_length_deserializer_test() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let test_cases = [
            (
                r#"{"protocol_state":{"body":{"consensus_state":{"blockchain_length":"42""#,
                42u32,
            ),
            (
                r#"{
                        "protocol_state": {
                            "body": {
                                "consensus_state": {
                                    "blockchain_length": "42"
                     "#,
                42,
            ),
            (
                r#"{"protocol_state":
                        {    "body"    :    {
                            "consensus_state"   :     { "blockchain_length":    "42""#,
                42,
            ),
            (
                r#"{"consensus_state": {
                            "blockchain_length":
                            "42"
                        }"#,
                42,
            ),
        ];

        for (i, (json_content, expected_length)) in test_cases.iter().enumerate() {
            let test_path = format!("{}/test-{i}.json", dir.path().display());
            write(&test_path, json_content)?;

            let blockchain_length = BlockchainLength::from_path(Path::new(&test_path))?;
            assert_eq!(blockchain_length, *expected_length);
        }
        Ok(())
    }

    #[test]
    fn blockchain_length_v1() -> anyhow::Result<()> {
        for path in
            glob::glob("./tests/data/canonical_chain_discovery/contiguous/*.json")?.flatten()
        {
            let blockchain_length = BlockchainLength::from_path(&path)?;
            let block = PrecomputedBlock::parse_file(&path, PcbVersion::V1)?;
            assert_eq!(blockchain_length, block.blockchain_length());
        }
        Ok(())
    }

    #[test]
    fn blockchain_length_v2() -> anyhow::Result<()> {
        for path in glob::glob("./tests/data/berkeley/sequential/*.json")?.flatten() {
            let blockchain_length = BlockchainLength::from_path(&path)?;
            let block = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;
            assert_eq!(blockchain_length, block.blockchain_length());
        }
        Ok(())
    }
}
