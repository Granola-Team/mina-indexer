use std::{fs::remove_dir_all, collections::HashMap, path::PathBuf};
use mina_indexer::{
    block::{parser::BlockParser, store::BlockStore},
    store::IndexerStore,
    MAINNET_CANONICAL_THRESHOLD,
};
use tokio::time::Instant;
use crate::helpers::setup_new_db_dir;

#[tokio::test]
async fn rocksdb() {
    let store_dir = setup_new_db_dir("./block-store-test");
    let log_dir = &PathBuf::from("./tests/data/sequential_blocks");

    let db = IndexerStore::new(&store_dir).unwrap();
    let mut bp = BlockParser::new(log_dir, MAINNET_CANONICAL_THRESHOLD).unwrap();

    let mut blocks = HashMap::new();

    let mut n = 0;
    let adding = Instant::now();
    while let Some(block) = bp.next_block().unwrap() {
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

    remove_dir_all(store_dir).unwrap();
}
