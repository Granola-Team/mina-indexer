use crate::block::precomputed::PrecomputedBlock;
use std::path::PathBuf;

pub struct GenesisBlock(PrecomputedBlock);

impl GenesisBlock {
    /// Creates the mainnet genesis block as a PCB
    pub fn new() -> anyhow::Result<Self> {
        let genesis_block_path: PathBuf = concat!(env!("PWD"), "/tests/data/genesis_blocks/mainnet-1-3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ.json").into();
        Ok(Self(PrecomputedBlock::parse_file(&genesis_block_path)?))
    }
}

impl GenesisBlock {
    pub fn to_precomputed(self) -> PrecomputedBlock {
        self.0
    }
}
