use mina_indexer::{block::parser::BlockParser, MAINNET_CANONICAL_THRESHOLD};
use std::path::PathBuf;

#[tokio::test]
async fn gaps() {
    let blocks_dir = PathBuf::from("./tests/data/canonical_chain_discovery/gaps");
    let mut block_parser = BlockParser::new(&blocks_dir, MAINNET_CANONICAL_THRESHOLD).unwrap();

    while let Some(precomputed_block) = block_parser.next().await.unwrap() {
        println!(
            "length: {}, hash: {}",
            precomputed_block.blockchain_length.unwrap_or(0),
            precomputed_block.state_hash
        );
    }

    // only length 2 is known to be canonical
    assert_eq!(block_parser.num_canonical, 1);
    assert_eq!(block_parser.total_num_blocks, 25);
}

#[tokio::test]
async fn contiguous() {
    let blocks_dir = PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");
    let mut block_parser = BlockParser::new(&blocks_dir, MAINNET_CANONICAL_THRESHOLD).unwrap();

    while let Some(precomputed_block) = block_parser.next().await.unwrap() {
        println!(
            "length: {}, hash: {}",
            precomputed_block.blockchain_length.unwrap_or(0),
            precomputed_block.state_hash
        );
    }

    // lengths 2..11 are known to be canonical
    assert_eq!(block_parser.num_canonical, 10);
    assert_eq!(block_parser.total_num_blocks, 20);
}

#[tokio::test]
async fn missing_parent() {
    let blocks_dir = PathBuf::from("./tests/data/canonical_chain_discovery/missing_parent");
    let mut block_parser = BlockParser::new(&blocks_dir, MAINNET_CANONICAL_THRESHOLD).unwrap();

    while let Some(precomputed_block) = block_parser.next().await.unwrap() {
        println!(
            "length: {}, hash: {}",
            precomputed_block.blockchain_length.unwrap_or(0),
            precomputed_block.state_hash
        );
    }

    // mainnet-105500-3NKvv2iBAPhZ8SRCxQEuGTgqTYuFXd2WVANXW6pcsR8pdzLuUj7C.json
    // which should be present in the canonical chain, is absent from the collection.
    // Therefore we can't determine the canonical tip from either block with length 105501.
    // Thus, the highest canonical tip we can find is
    // mainnet-105490-3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC.json
    // and the best canonical consists of blocks with lengths 105487, 105488, 105489, 105490.
    assert_eq!(block_parser.num_canonical, 4);
    assert_eq!(block_parser.total_num_blocks, 35)
}
