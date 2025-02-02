use super::precomputed::PcbVersion;
use crate::block::precomputed::PrecomputedBlock;

#[derive(Debug)]
pub struct GenesisBlock(pub PrecomputedBlock, pub u64);

pub const GENESIS_MAINNET_BLOCK_CONTENTS: &str = include_str!(
    "../../data/genesis_blocks/mainnet-1-3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ.json"
);

pub const GENESIS_HARDFORK_BLOCK_CONTENTS: &str = include_str!(
    "../../data/genesis_blocks/mainnet-359605-3NK4BpDSekaqsG6tx8Nse2zJchRft2JpnbvMiog55WCr5xJZaKeP.json"
);

impl GenesisBlock {
    /// Creates the v1 (pre-hardfork) mainnet genesis block as a PCB
    pub fn new_v1() -> anyhow::Result<Self> {
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

    /// Creates the v2 (hardfork) mainnet genesis block as a PCB
    pub fn new_v2() -> anyhow::Result<Self> {
        let contents = GENESIS_HARDFORK_BLOCK_CONTENTS.as_bytes().to_vec();
        let size = contents.len() as u64;
        let network = "mainnet";
        let blockchain_length: u32 = 359605;
        let state_hash = "3NK4BpDSekaqsG6tx8Nse2zJchRft2JpnbvMiog55WCr5xJZaKeP";

        Ok(Self(
            PrecomputedBlock::new(
                network,
                blockchain_length,
                state_hash,
                contents,
                PcbVersion::V2,
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
    fn parse_genesis_block_v1() -> anyhow::Result<()> {
        let block = GenesisBlock::new_v1()?;
        assert_eq!(
            block.0.state_hash().0,
            "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ",
        );
        Ok(())
    }

    #[test]
    fn parse_genesis_block_v2() -> anyhow::Result<()> {
        let block = GenesisBlock::new_v2()?;
        assert_eq!(
            block.0.state_hash().0,
            "3NK4BpDSekaqsG6tx8Nse2zJchRft2JpnbvMiog55WCr5xJZaKeP",
        );
        Ok(())
    }
}
