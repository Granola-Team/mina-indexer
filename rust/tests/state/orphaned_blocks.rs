use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::{parser::BlockParser, store::BlockStore},
    ledger::genesis::GenesisRoot,
    state::IndexerState,
    store::IndexerStore,
};
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn not_added_to_witness_tree() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("orphaned-blocks")?;
    let log_dir = PathBuf::from("./tests/data/sequential_blocks");
    let mut block_parser = BlockParser::new_with_canonical_chain_discovery(&log_dir, 10, 10)?;
    let indexer_store = Arc::new(IndexerStore::new(store_dir.path())?);
    let genesis_contents = include_str!("../data/genesis_ledgers/mainnet.json");
    let genesis_ledger = serde_json::from_str::<GenesisRoot>(genesis_contents)?;
    let mut state = IndexerState::new(genesis_ledger.clone().into(), indexer_store.clone(), 10)?;

    // add all blocks to the state
    state.add_blocks(&mut block_parser)?;

    // This block is deep canonical:
    // 0: mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json
    // It is included in the witness tree's diff map and the block store
    let state_hash0 = "3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT".into();
    assert!(state.diffs_map.contains_key(&state_hash0));
    assert_eq!(
        indexer_store.get_block(&state_hash0)?.unwrap().state_hash,
        state_hash0.0
    );

    // These two blocks are orphaned:
    // 1: mainnet-105489-3NLFXtdzaFW2WX6KgrxMjL4enE4pCa9hAsVUPm47PT6337SXgBGh.json
    // 2: mainnet-105489-3NLUfaHDcyt9KsYxi1xsSdYE369GAduLxVgRUDE7RuFgSXQBphDK.json
    let state_hash1 = "3NLFXtdzaFW2WX6KgrxMjL4enE4pCa9hAsVUPm47PT6337SXgBGh".into();
    assert!(!state.diffs_map.contains_key(&state_hash1));
    assert_eq!(
        indexer_store.get_block(&state_hash1)?.unwrap().state_hash,
        state_hash1.0
    );

    let state_hash2 = "3NLUfaHDcyt9KsYxi1xsSdYE369GAduLxVgRUDE7RuFgSXQBphDK".into();
    assert!(!state.diffs_map.contains_key(&state_hash2));
    assert_eq!(
        indexer_store.get_block(&state_hash2)?.unwrap().state_hash,
        state_hash2.0
    );

    Ok(())
}
