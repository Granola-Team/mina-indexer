//! Indexer blockchain length type

use crate::{block::precomputed::PcbVersion, constants::HARDFORK_GENESIS_BLOCKCHAIN_LENGTH};
use anyhow::bail;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    fs::File,
    io::{BufReader, Read},
    path::Path,
    str::FromStr,
};

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct BlockchainLength(pub u32);

/////////////////
// conversions //
/////////////////

impl From<u32> for BlockchainLength {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl FromStr for BlockchainLength {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let blockchain_length = s.parse()?;
        Ok(Self(blockchain_length))
    }
}

/// If the blockchain length is greater than or equal to the hardfork genesis
/// blockchain length, return version 2, otherwise return version 1.
impl From<BlockchainLength> for PcbVersion {
    fn from(blockchain_length: BlockchainLength) -> Self {
        if blockchain_length.0 >= HARDFORK_GENESIS_BLOCKCHAIN_LENGTH {
            PcbVersion::V2
        } else {
            PcbVersion::V1
        }
    }
}

impl BlockchainLength {
    pub fn from_path(path: &Path) -> anyhow::Result<u32> {
        const BUFFER_CAPACITY: usize = 1000;

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut buffer = String::with_capacity(BUFFER_CAPACITY);

        // Limit the reader to read only the first BUFFER_CAPACITY bytes
        reader
            .take(BUFFER_CAPACITY as u64)
            .read_to_string(&mut buffer)?;

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

///////////
// serde //
///////////

impl Serialize for BlockchainLength {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        crate::utility::serde::to_str(self, serializer)
    }
}

impl<'de> Deserialize<'de> for BlockchainLength {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        crate::utility::serde::from_str(deserializer)
    }
}

/////////////
// display //
/////////////

impl Display for BlockchainLength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::BlockchainLength;
    use crate::block::precomputed::{PcbVersion, PrecomputedBlock};
    use std::{fs::write, path::Path};
    use tempfile::TempDir;

    #[test]
    fn roundtrip() -> anyhow::Result<()> {
        let len = BlockchainLength::default();
        let len_str = len.to_string();

        // serialize
        let ser = serde_json::to_vec(&len)?;

        // deserialize
        let res: BlockchainLength = serde_json::from_slice(&ser)?;

        // roundtrip
        assert_eq!(len, res);

        // same serialization as string
        assert_eq!(ser, serde_json::to_vec(&len_str)?);

        Ok(())
    }

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
