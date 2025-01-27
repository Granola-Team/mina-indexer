use crate::helpers::{state::*, store::*};
use mina_indexer::block::{parser::BlockParser, precomputed::PcbVersion, store::BlockStore};
use std::path::PathBuf;

#[tokio::test]
async fn not_added_to_witness_tree() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("orphaned-blocks")?;
    let block_dir = PathBuf::from("./tests/data/sequential_blocks");

    let mut block_parser =
        BlockParser::new_with_canonical_chain_discovery(&block_dir, PcbVersion::V1, 10, false, 10)
            .await?;

    let mut state = mainnet_genesis_state(store_dir.as_ref())?;

    // add all blocks to the state
    state.add_blocks(&mut block_parser).await?;

    let store = state.indexer_store.as_ref().unwrap();

    // This block is deep canonical:
    // 0: mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json
    // It is included in the witness tree's diff map and the block store
    let state_hash0 = "3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT".into();
    assert!(state.diffs_map.contains_key(&state_hash0));
    assert_eq!(
        store.get_block(&state_hash0)?.unwrap().0.state_hash().0,
        state_hash0.0
    );

    // These two blocks are orphaned:
    // 1: mainnet-105489-3NLFXtdzaFW2WX6KgrxMjL4enE4pCa9hAsVUPm47PT6337SXgBGh.json
    // 2: mainnet-105489-3NLUfaHDcyt9KsYxi1xsSdYE369GAduLxVgRUDE7RuFgSXQBphDK.json
    let state_hash1 = "3NLFXtdzaFW2WX6KgrxMjL4enE4pCa9hAsVUPm47PT6337SXgBGh".into();
    assert!(!state.diffs_map.contains_key(&state_hash1));
    assert_eq!(
        store.get_block(&state_hash1)?.unwrap().0.state_hash().0,
        state_hash1.0
    );

    let state_hash2 = "3NLUfaHDcyt9KsYxi1xsSdYE369GAduLxVgRUDE7RuFgSXQBphDK".into();
    assert!(!state.diffs_map.contains_key(&state_hash2));
    assert_eq!(
        store.get_block(&state_hash2)?.unwrap().0.state_hash().0,
        state_hash2.0
    );

    Ok(())
}
