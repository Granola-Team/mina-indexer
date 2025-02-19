use std::path::Path;

pub type PrecomputedBlock = crate::block::precomputed::v2::BlockFileV2;

/// Parse
pub fn parse_file<P: AsRef<Path>>(path: P) -> anyhow::Result<PrecomputedBlock> {
    let contents = std::fs::read(path)?;
    Ok(serde_json::from_slice(&contents)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{block::extract_block_height, constants::HARDFORK_GENESIS_BLOCKCHAIN_LENGTH};
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
        // sequential blocks
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

        // non-sequential blocks
        glob::glob("./tests/data/berkeley/non_sequential_blocks/berkeley-*-*.json")?.for_each(
            |path| {
                if let Ok(ref path) = path {
                    if let Err(e) = parse_file(path) {
                        panic!(
                            "Error parsing block {}: {}",
                            path.file_name().unwrap().to_str().unwrap(),
                            e
                        )
                    }
                }
            },
        );

        // misc hardfork blocks
        glob::glob("./tests/data/misc_blocks/mainnet-*-*.json")?
            .flatten()
            .filter(|path| extract_block_height(path) >= HARDFORK_GENESIS_BLOCKCHAIN_LENGTH)
            .for_each(|path| {
                if let Err(e) = parse_file(&path) {
                    panic!(
                        "Error parsing block {}: {}",
                        path.file_name().unwrap().to_str().unwrap(),
                        e
                    )
                }
            });

        Ok(())
    }
}
