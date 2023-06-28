use mina_indexer::{
    block::parser::BlockParser,
    state::{ExtensionType, IndexerState},
};
use std::path::PathBuf;
use tokio::fs::remove_dir_all;

/// Adds the same block twice, second time fails
#[tokio::test]
async fn test() {
    let block_store_dir = PathBuf::from("./test_block_store");
    let log_dir = PathBuf::from("./tests/data/sequential_blocks");
    let mut block_parser = BlockParser::new_testing(&log_dir).unwrap();

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
    let mut state =
        IndexerState::new_testing(&root_block, None, Some(&block_store_dir), None).unwrap();

    // add block for the first time
    let extension_type = state.add_block(&block0, false).unwrap();
    assert_eq!(extension_type, ExtensionType::DanglingNew);

    println!("=== Before state ===");
    print!("{state:?}");

    println!("Root:     {}", state.root_branch.len());
    println!("Dangling: {}", state.dangling_branches.len());

    // 1 block in the root branch
    // 1 blovk in the 0th dangling branch
    assert_eq!(state.root_branch.clone().len(), 1);
    assert_eq!(state.dangling_branches.len(), 1);
    assert_eq!(state.dangling_branches.get(0).unwrap().len(), 1);

    // block not added again
    assert_eq!(
        state.add_block(&block1, true).unwrap(),
        ExtensionType::BlockNotAdded
    );

    // the block is not added to the state
    println!("Root:     {}", state.root_branch.clone().len());
    println!("Dangling: {}", state.dangling_branches.len());

    assert_eq!(state.root_branch.len(), 1);
    assert_eq!(state.dangling_branches.len(), 1);
    assert_eq!(state.dangling_branches.get(0).unwrap().len(), 1);

    remove_dir_all(block_store_dir).await.unwrap();
}
