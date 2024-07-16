use mina_indexer::{
    block::{
        parser::BlockParser,
        precomputed::{PcbVersion, PrecomputedBlock},
    },
    constants::*,
};
use std::path::PathBuf;

#[tokio::test]
async fn gaps() -> anyhow::Result<()> {
    let blocks_dir = PathBuf::from("./tests/data/canonical_chain_discovery/gaps");
    let mut block_parser = BlockParser::new_with_canonical_chain_discovery(
        &blocks_dir,
        PcbVersion::V1,
        MAINNET_CANONICAL_THRESHOLD,
        BLOCK_REPORTING_FREQ_NUM,
    )
    .await?;

    while let Some((block, _)) = block_parser.next_block().await? {
        let block: PrecomputedBlock = block.into();
        println!("{}", block.summary());
    }

    // only length 2 is known to be canonical
    assert_eq!(block_parser.total_num_blocks, 26);
    assert_eq!(block_parser.num_deep_canonical_blocks, 1);
    Ok(())
}

#[tokio::test]
async fn contiguous() -> anyhow::Result<()> {
    let blocks_dir = PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");
    let mut block_parser = BlockParser::new_with_canonical_chain_discovery(
        &blocks_dir,
        PcbVersion::V1,
        MAINNET_CANONICAL_THRESHOLD,
        BLOCK_REPORTING_FREQ_NUM,
    )
    .await?;

    while let Some((block, _)) = block_parser.next_block().await? {
        let block: PrecomputedBlock = block.into();
        println!("{}", block.summary());
    }

    // lengths 2..11 are known to be canonical
    assert_eq!(block_parser.total_num_blocks, 20);
    assert_eq!(block_parser.num_deep_canonical_blocks, 10);
    Ok(())
}

#[tokio::test]
async fn missing_parent() -> anyhow::Result<()> {
    let blocks_dir = PathBuf::from("./tests/data/canonical_chain_discovery/missing_parent");
    let mut block_parser = BlockParser::new_with_canonical_chain_discovery(
        &blocks_dir,
        PcbVersion::V1,
        MAINNET_CANONICAL_THRESHOLD,
        BLOCK_REPORTING_FREQ_NUM,
    )
    .await?;

    while let Some((block, _)) = block_parser.next_block().await? {
        let block: PrecomputedBlock = block.into();
        println!("{}", block.summary());
    }

    // mainnet-105500-3NKvv2iBAPhZ8SRCxQEuGTgqTYuFXd2WVANXW6pcsR8pdzLuUj7C.json
    // which should be present in the canonical chain, is absent from the
    // collection. Therefore we can't determine the canonical root from either
    // block with length 105501. Thus, the highest canonical root we can find is
    // mainnet-105490-3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC.json
    // and the best canonical consists of blocks with lengths 105487, 105488,
    // 105489, 105490.
    assert_eq!(block_parser.total_num_blocks, 35);
    assert_eq!(block_parser.num_deep_canonical_blocks, 4);
    Ok(())
}

#[tokio::test]
async fn one_block() -> anyhow::Result<()> {
    let blocks_dir = PathBuf::from("./tests/data/canonical_chain_discovery/one_block");
    let block_parser = BlockParser::new_with_canonical_chain_discovery(
        &blocks_dir,
        PcbVersion::V1,
        MAINNET_CANONICAL_THRESHOLD,
        BLOCK_REPORTING_FREQ_NUM,
    )
    .await?;

    assert_eq!(block_parser.total_num_blocks, 1);
    assert_eq!(block_parser.num_deep_canonical_blocks, 0);
    Ok(())
}

#[tokio::test]
async fn canonical_threshold() -> anyhow::Result<()> {
    let canonical_threshold = 2;
    let blocks_dir = PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");
    let mut block_parser = BlockParser::new_with_canonical_chain_discovery(
        &blocks_dir,
        PcbVersion::V1,
        canonical_threshold,
        BLOCK_REPORTING_FREQ_NUM,
    )
    .await?;

    while let Some((block, _)) = block_parser.next_block().await? {
        let block: PrecomputedBlock = block.into();
        println!("{}", block.summary());
    }

    // lengths 2..19 are known to be canonical
    assert_eq!(
        block_parser.num_deep_canonical_blocks,
        block_parser.total_num_blocks - canonical_threshold
    );
    assert_eq!(block_parser.total_num_blocks, 20);
    Ok(())
}
