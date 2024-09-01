use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::{
        parser::BlockParser,
        precomputed::{PcbVersion, PrecomputedBlock},
        store::BlockStore,
    },
    constants::*,
    store::IndexerStore,
};
use std::path::PathBuf;

#[tokio::test]
async fn add_and_get() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("blocks-at-height")?;
    let block_dir = &PathBuf::from("./tests/data/sequential_blocks");

    let db = IndexerStore::new(store_dir.path())?;
    let mut bp = BlockParser::new_with_canonical_chain_discovery(
        block_dir,
        PcbVersion::V1,
        MAINNET_CANONICAL_THRESHOLD,
        false,
        BLOCK_REPORTING_FREQ_NUM,
    )
    .await?;

    while let Some((block, block_bytes)) = bp.next_block().await? {
        let block: PrecomputedBlock = block.into();
        db.add_block(&block, block_bytes)?;
        println!("{}", block.summary());
    }

    for state_hash in db.get_blocks_at_height(105489)? {
        println!("{state_hash}");
    }

    assert_eq!(db.get_blocks_at_height(105489)?.len(), 3);
    Ok(())
}
