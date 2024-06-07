use super::precomputed::PcbVersion;
use crate::block::precomputed::PrecomputedBlock;
use log::info;
use std::path::PathBuf;

#[derive(Debug)]
pub struct GenesisBlock(pub PrecomputedBlock, pub u64);

impl GenesisBlock {
    /// Creates the mainnet genesis block as a PCB
    pub fn new() -> anyhow::Result<Self> {
        let genesis_block_path: PathBuf =
            concat!(env!("PWD"), "/tests/data/genesis_blocks/mainnet-1-3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ.json").into();
        info!(
            "Looking for genesis block at {}",
            genesis_block_path.display()
        );
        assert!(
            genesis_block_path.is_file(),
            "Genesis block does not exist at {}",
            genesis_block_path.display()
        );
        Ok(Self(
            PrecomputedBlock::parse_file(&genesis_block_path, PcbVersion::V1)?,
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
