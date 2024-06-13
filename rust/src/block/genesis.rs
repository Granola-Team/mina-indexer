use super::precomputed::PcbVersion;
use crate::block::precomputed::PrecomputedBlock;

#[derive(Debug)]
pub struct GenesisBlock(pub PrecomputedBlock, pub u64);

pub const GENESIS_MAINNET_BLOCK_CONTENTS: &str = include_str!("../../data/genesis_blocks/mainnet-1-3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ.json");
impl GenesisBlock {
    /// Creates the mainnet genesis block as a PCB
    pub fn new() -> anyhow::Result<Self> {
        let contents = GENESIS_MAINNET_BLOCK_CONTENTS.as_bytes().to_vec();
        let size = contents.len() as u64;
        let network = "mainnet";
        let blockchain_length: u32 = 1;
        let state_hash = "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ";

        Ok(Self(
            PrecomputedBlock::new(
                network,
                blockchain_length,
                state_hash,
                contents,
                PcbVersion::V1,
            )?,
            size,
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
