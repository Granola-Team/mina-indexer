use crate::helpers::store::*;
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
    let store_dir = setup_new_db_dir("blocks-at-slot")?;
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
    }

    assert_eq!(db.get_blocks_at_slot(155140)?.len(), 3);
    assert_eq!(
        db.get_next_global_slot_produced(&MAINNET_GENESIS_HASH.into(), 155157)?
            .unwrap(),
        155158
    );
    assert_eq!(
        db.get_prev_global_slot_produced(&MAINNET_GENESIS_HASH.into(), 155157)?,
        155156
    );

    Ok(())
}
