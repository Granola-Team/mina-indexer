use super::BlockHash;
use anyhow::bail;
use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

pub struct GenesisStateHash(BlockHash);

impl GenesisStateHash {
    pub fn from_path(path: &Path) -> anyhow::Result<BlockHash> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut buffer = String::with_capacity(400);

        // Limit the reader to read only the first 400 bytes
        reader.take(400).read_to_string(&mut buffer)?;

        // Locate "genesis_state_hash" within the buffer
        let gen_state_hash_key = "\"genesis_state_hash\"";
        if let Some(hash_pos) = buffer.find(gen_state_hash_key) {
            let hash_start = hash_pos + gen_state_hash_key.len();

            // Find the first quote after the colon
            if let Some(quote_start) = buffer[hash_start..].find('"') {
                let start = hash_start + quote_start + 1;
                if start + BlockHash::LEN <= buffer.len() {
                    let genesis_state_hash = &buffer[start..][..BlockHash::LEN];
                    return Ok(genesis_state_hash.into());
                }
            }
        }
        bail!("Failed to find genesis_state_hash in the file")
    }
}

impl From<GenesisStateHash> for BlockHash {
    fn from(value: GenesisStateHash) -> Self {
        value.0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        block::precomputed::{PcbVersion, PrecomputedBlock},
        chain::Network,
    };
    use std::fs::write;
    use tempfile::TempDir;

    #[test]
    fn genesis_state_hash_deserializer_test() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let test_cases = [
            (
                r#"{"body":{"genesis_state_hash":"3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw""#,
                "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw",
            ),
            (
                r#"{
                        "protocol_state": {
                            "body": {
                                "genesis_state_hash": "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw"
                     "#,
                "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw",
            ),
            (
                r#"{"protocol_state":
                        {    "body"     :
                        {    "genesis_state_hash"    :    "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw""#,
                "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw",
            ),
            (
                r#"{"protocol_state": {    "body" : {
                            "genesis_state_hash":
                            "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw"
                        "#,
                "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw",
            ),
        ];

        for (i, (json_content, expected_hash)) in test_cases.iter().enumerate() {
            let test_path = format!(
                "{}/{}-{i}-{}.json",
                dir.path().display(),
                Network::default(),
                BlockHash::default()
            );
            write(&test_path, json_content)?;

            let genesis_state_hash = GenesisStateHash::from_path(Path::new(&test_path))?;
            assert_eq!(genesis_state_hash.0, *expected_hash);
        }
        Ok(())
    }

    #[test]
    fn genesis_state_hash_v1() -> anyhow::Result<()> {
        for path in
            glob::glob("./tests/data/canonical_chain_discovery/contiguous/*.json")?.flatten()
        {
            let genesis_state_hash = GenesisStateHash::from_path(&path)?;
            let block = PrecomputedBlock::parse_file(&path, PcbVersion::V1)?;
            assert_eq!(genesis_state_hash, block.genesis_state_hash());
        }
        Ok(())
    }

    #[test]
    fn genesis_state_hash_v2() -> anyhow::Result<()> {
        for path in glob::glob("./tests/data/berkeley/sequential/*.json")?.flatten() {
            let genesis_state_hash = GenesisStateHash::from_path(&path)?;
            let block = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;
            assert_eq!(genesis_state_hash, block.genesis_state_hash());
        }
        Ok(())
    }
}
