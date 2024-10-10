use crate::block::BlockHash;
use anyhow::bail;
use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

#[derive(PartialEq, Eq)]
pub struct PreviousStateHash(BlockHash);

impl PreviousStateHash {
    pub fn from_path(path: &Path) -> anyhow::Result<BlockHash> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut buffer = String::with_capacity(200);

        // Limit the reader to read only the first 200 bytes
        reader.take(200).read_to_string(&mut buffer)?;

        // Locate "previous_state_hash" within the buffer
        let prev_state_hash_key = "\"previous_state_hash\"";
        if let Some(hash_pos) = buffer.find(prev_state_hash_key) {
            let hash_start = hash_pos + prev_state_hash_key.len();

            // Find the first quote after the colon
            if let Some(quote_start) = buffer[hash_start..].find('"') {
                let start = hash_start + quote_start + 1;
                if start + BlockHash::LEN <= buffer.len() {
                    let previous_state_hash = &buffer[start..][..BlockHash::LEN];
                    return Ok(previous_state_hash.into());
                }
            }
        }
        bail!("Failed to find previous_state_hash in the file")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::block::precomputed::{PcbVersion, PrecomputedBlock};
    use std::fs::write;
    use tempfile::TempDir;

    #[test]
    fn previous_state_hash_deserializer_test() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let test_cases = [
            (
                r#"{"protocol_state":{"previous_state_hash":"3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw""#,
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
            let test_path = format!("{}/test-{i}.json", dir.path().display());
            write(&test_path, json_content)?;

            let previous_state_hash = PreviousStateHash::from_path(Path::new(&test_path))?;
            assert_eq!(previous_state_hash.0, *expected_hash);
        }
        Ok(())
    }

    #[test]
    fn previous_state_hash_v1() -> anyhow::Result<()> {
        for path in
            glob::glob("./tests/data/canonical_chain_discovery/contiguous/*.json")?.flatten()
        {
            let previous_state_hash = PreviousStateHash::from_path(&path)?;
            let block = PrecomputedBlock::parse_file(&path, PcbVersion::V1)?;
            assert_eq!(previous_state_hash, block.previous_state_hash());
        }
        Ok(())
    }

    #[test]
    fn previous_state_hash_v2() -> anyhow::Result<()> {
        for path in glob::glob("./tests/data/berkeley/sequential/*.json")?.flatten() {
            let previous_state_hash = PreviousStateHash::from_path(&path)?;
            let block = PrecomputedBlock::parse_file(&path, PcbVersion::V2)?;
            assert_eq!(previous_state_hash, block.previous_state_hash());
        }
        Ok(())
    }
}
