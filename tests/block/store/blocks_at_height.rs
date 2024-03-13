use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::{parser::BlockParser, precomputed::PrecomputedBlock, store::BlockStore},
    constants::*,
    store::IndexerStore,
};
use std::path::PathBuf;

#[test]
fn add_and_get() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("blocks-at-height")?;
    let block_dir = &PathBuf::from("./tests/data/sequential_blocks");

    let db = IndexerStore::new(store_dir.path())?;
    let mut bp = BlockParser::new_with_canonical_chain_discovery(
        block_dir,
        MAINNET_CANONICAL_THRESHOLD,
        BLOCK_REPORTING_FREQ_NUM,
    )?;

    while let Some(block) = bp.next_block()? {
        let block: PrecomputedBlock = block.into();
        db.add_block(&block)?;
        println!("{}: {}", block.blockchain_length, block.state_hash);
    }

    for block in db.get_blocks_at_height(105489)? {
        println!("{}: {}", block.blockchain_length, block.state_hash);
    }

    assert_eq!(db.get_blocks_at_height(105489)?.len(), 3);
    Ok(())
}
