use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::{parser::BlockParser, precomputed::PrecomputedBlock, store::BlockStore},
    constants::*,
    store::IndexerStore,
};
use std::{collections::HashMap, path::PathBuf, time::Instant};

#[test]
fn add_and_get() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("block-store-db")?;
    let blocks_dir = &PathBuf::from("./tests/data/sequential_blocks");

    let db = IndexerStore::new(store_dir.path())?;
    let mut bp = BlockParser::new_with_canonical_chain_discovery(
        blocks_dir,
        MAINNET_CANONICAL_THRESHOLD,
        BLOCK_REPORTING_FREQ_NUM,
    )?;

    let mut blocks = HashMap::new();

    let mut n = 0;
    let adding = Instant::now();
    while let Some((block, _)) = bp.next_block()? {
        let block: PrecomputedBlock = block.into();
        let state_hash = block.state_hash.clone();

        db.add_block(&block)?;
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
    Ok(())
}
