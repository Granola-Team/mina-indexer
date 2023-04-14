use std::path::PathBuf;

use mina_indexer::{block::parser::BlockParser, state::IndexerState};

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

    let mut state = IndexerState::new(&block0, None).unwrap();

    // precomputed blocks are obviously the same (they're clones!)
    assert_eq!(block0, block1);

    println!("=== Dangling Branch 0 ===");
    let mut tree = String::new();
    state
        .dangling_branches
        .get(0)
        .unwrap()
        .branches
        .write_formatted(&mut tree)
        .unwrap();
    println!("{tree}");

    // add_block throws an error
    match state.add_block(&block1) {
        Ok(_) => panic!(),
        Err(err) => println!("{:?}", err),
    }

    // the block is not addes
    assert_eq!(state.dangling_branches.len(), 1);
    assert_eq!(state.dangling_branches.get(0).unwrap().branches.height(), 1);
}
