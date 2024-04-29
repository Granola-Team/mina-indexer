use super::precomputed::PcbVersion;
use crate::block::precomputed::PrecomputedBlock;
use std::path::PathBuf;

#[derive(Debug)]
pub struct GenesisBlock(pub PrecomputedBlock, pub u64);

impl GenesisBlock {
    /// Creates the mainnet genesis block as a PCB
    pub fn new() -> anyhow::Result<Self> {
        let genesis_block_path: PathBuf = match std::env::var("RELEASE").unwrap_or_default().as_str() {
            // TODO: This should be handled up the call stack. Writing here so it builds correctly
            "production" => {
                "/share/mina-indexer/data/mainnet-1-3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ.json".into()
            },
            _ => {
                concat!(env!("PWD"), "/tests/data/genesis_blocks/mainnet-1-3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ.json").into()
            }
        };
        Ok(Self(
            PrecomputedBlock::parse_file(&genesis_block_path, PcbVersion(0))?,
            genesis_block_path.metadata().unwrap().len(),
        ))
    }
}

impl GenesisBlock {
    pub fn to_precomputed(self) -> PrecomputedBlock {
        self.0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_genesis_block() -> anyhow::Result<()> {
        match GenesisBlock::new() {
            Ok(block) => {
                let state_hash = block.0.state_hash().0;
                assert_eq!(
                    "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ",
                    state_hash
                );
            }
            Err(e) => {
                anyhow::bail!(e);
            }
        }
        Ok(())
    }
}
