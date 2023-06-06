use std::path::PathBuf;

use mina_indexer::{
    block::{parser::BlockParser, BlockHash},
    state::IndexerState,
};

/// Adds the same block
#[tokio::test]
async fn test() {
    let log_dir = PathBuf::from("./tests/data/beautified_sequential_blocks");
    let mut block_parser = BlockParser::new(&log_dir).unwrap();

    // block0 = mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json
    let block0 = block_parser
        .get_precomputed_block("3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT")
        .await
        .unwrap();
    assert_eq!(
        block0.state_hash,
        "3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT".to_owned()
    );

    // block1 = mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json
    let block1 = block0.clone();
    let mut state = IndexerState::new(BlockHash(block0.state_hash), None, None).unwrap();

    // println!("=== Root Branch ===");
    let mut tree = String::new();
    state
        .root_branch
        .clone()
        .unwrap()
        .branches
        .write_formatted(&mut tree)
        .unwrap();

    println!("Root:     {}", state.root_branch.clone().unwrap().len());
    println!("Dangling: {}", state.dangling_branches.len());

    assert_eq!(state.root_branch.clone().unwrap().len(), 1);
    assert_eq!(state.dangling_branches.len(), 0);

    // throws err
    match state.add_block(&block1) {
        Ok(ext) => println!("Extension type: {ext:?}"),
        Err(err) => println!("{err:?}"),
    }

    // the block is not added to root or dangling
    println!("Root:     {}", state.root_branch.clone().unwrap().len());
    println!("Dangling: {}", state.dangling_branches.len());

    assert_eq!(state.root_branch.unwrap().len(), 1);
    assert_eq!(state.dangling_branches.len(), 0);
}
