use std::{collections::HashMap, path::PathBuf};

use mina_indexer::{
    block::{parser::BlockParser, store::BlockStore},
    store::IndexerStore,
};
use tokio::time::Instant;

#[tokio::test]
async fn rocksdb() {
    let store_dir = &PathBuf::from("./block-store-test");
    let log_dir = &PathBuf::from("./tests/data/sequential_blocks");

    let db = IndexerStore::new(store_dir).unwrap();
    let mut bp = BlockParser::new(log_dir).unwrap();
    let mut blocks = HashMap::new();

    let mut n = 0;
    let adding = Instant::now();
    while let Some(block) = bp.next().await.unwrap() {
        let state_hash = block.state_hash.clone();
        db.add_block(&block).unwrap();
        blocks.insert(state_hash.clone(), block);
        println!("Added {:?}", &state_hash);
        n += 1;
    }
    let add_time = adding.elapsed();

    let fetching = Instant::now();
    for (state_hash, block) in blocks.iter() {
        assert_eq!(block, blocks.get(state_hash).unwrap());
    }

    println!("\n~~~~~~~~~~~~~~~~~~");
    println!("~~~ Benchmarks ~~~");
    println!("~~~~~~~~~~~~~~~~~~");
    println!("Number of blocks: {n}");
    println!("To insert all:    {add_time:?}");
    println!("To fetch all:     {:?}\n", fetching.elapsed());

    tokio::fs::remove_dir_all(store_dir).await.unwrap();
}
