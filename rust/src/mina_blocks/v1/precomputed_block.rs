use std::path::Path;

pub type PrecomputedBlock = crate::block::precomputed::v1::BlockFileV1;

pub fn parse_file<P: AsRef<Path>>(path: P) -> anyhow::Result<PrecomputedBlock> {
    let contents = std::fs::read(path)?;
    Ok(serde_json::from_slice(&contents)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize() -> anyhow::Result<()> {
        let path = "./tests/data/non_sequential_blocks/mainnet-40702-3NLkEG6S6Ra8Z1i5U5MPSNWV13hzQV8pYx1xBaeLDFN4EJhSuksw.json";
        let block = parse_file(path)?;

        println!("{}", serde_json::to_string_pretty(&block)?);
        Ok(())
    }
}
