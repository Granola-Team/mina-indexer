use std::path::PathBuf;

use mina_indexer::{
    block::{parser::BlockParser, BlockHash},
    state::{ExtensionType, IndexerState},
};
use tokio::fs::remove_dir_all;

/// Adds the same block twice, second time fails
#[tokio::test]
async fn test() {
    let block_store_dir = PathBuf::from("./test_block_store");
    let log_dir = PathBuf::from("./tests/data/beautified_sequential_blocks");
    let mut block_parser = BlockParser::new(&log_dir).unwrap();

    // root_block = mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json
    let root_block = block_parser
        .get_precomputed_block("3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT")
        .await
        .unwrap();
    assert_eq!(
        root_block.state_hash,
        "3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT".to_owned()
    );

    // block0 = mainnet-105491-3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3.json
    let block0 = block_parser
        .get_precomputed_block("3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3")
        .await
        .unwrap();
    assert_eq!(
        block0.state_hash,
        "3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3".to_owned()
    );

    // block1 = mainnet-105491-3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3.json
    let block1 = block0.clone();
    assert_eq!(
        block1.state_hash,
        "3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3".to_owned()
    );

    // initialize state
    let mut state = IndexerState::new(
        BlockHash(root_block.state_hash),
        None,
        Some(&block_store_dir),
    )
    .unwrap();

    // add block for the first time
    let extension_type = state.add_block(&block0).unwrap();
    assert_eq!(extension_type, ExtensionType::DanglingNew);

    println!("=== Before state ===");
    println!("{state:?}");

    println!("Root:     {}", state.root_branch.as_ref().unwrap().len());
    println!("Dangling: {}", state.dangling_branches.len());

    // 1 block in the root branch
    // 1 blovk in the 0th dangling branch
    assert_eq!(state.root_branch.clone().unwrap().len(), 1);
    assert_eq!(state.dangling_branches.len(), 1);
    assert_eq!(state.dangling_branches.get(0).unwrap().len(), 1);

    // throws err
    assert!(match state.add_block(&block1) {
        Ok(ext) => {
            println!("Extension type: {ext:?}");
            false
        }
        Err(err) => {
            println!("{err:?}");
            true
        }
    });

    // the block is not added to the state
    println!("Root:     {}", state.root_branch.clone().unwrap().len());
    println!("Dangling: {}", state.dangling_branches.len());

    assert_eq!(state.root_branch.unwrap().len(), 1);
    assert_eq!(state.dangling_branches.len(), 1);
    assert_eq!(state.dangling_branches.get(0).unwrap().len(), 1);

    remove_dir_all(block_store_dir).await.unwrap();
}
