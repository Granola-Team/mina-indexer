use std::{collections::HashMap, path::PathBuf};

use mina_indexer::block::{parser::BlockParser, store::BlockStore};

#[tokio::test]
async fn rocksdb() {
    let store_dir = &PathBuf::from("./block-store-test");
    let log_dir = &PathBuf::from("./tests/data/beautified_sequential_blocks");

    let db = BlockStore::new(store_dir).unwrap();
    let mut bp = BlockParser::new(log_dir).unwrap();
    let mut blocks = HashMap::new();

    while let Some(block) = bp.next().await.unwrap() {
        let state_hash = block.state_hash.clone();
        db.add_block(&block).unwrap();
        blocks.insert(state_hash.clone(), block);
        println!("Added {:?}", &state_hash);
    }

    for (state_hash, block) in blocks.iter() {
        assert_eq!(block, blocks.get(state_hash).unwrap());
    }

    tokio::fs::remove_dir_all(store_dir).await.unwrap();
}
