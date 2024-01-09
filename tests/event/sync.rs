use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::parser::BlockParser,
    state::{ledger::genesis::GenesisRoot, IndexerState},
    store::IndexerStore,
    MAINNET_CANONICAL_THRESHOLD, MAINNET_GENESIS_HASH, PRUNE_INTERVAL_DEFAULT,
};
use std::{fs::remove_dir_all, path::PathBuf, sync::Arc};

#[tokio::test]
async fn test() {
    let db_path = setup_new_db_dir("./test-event-sync");
    let log_dir = PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");
    let mut block_parser = BlockParser::new_testing(&log_dir).unwrap();
    let indexer_store = Arc::new(IndexerStore::new(&db_path).unwrap());
    let genesis_contents = include_str!("../data/genesis_ledgers/mainnet.json");
    let genesis_ledger = serde_json::from_str::<GenesisRoot>(genesis_contents)
        .unwrap()
        .ledger;
    let mut state0 = IndexerState::new(
        &MAINNET_GENESIS_HASH.into(),
        genesis_ledger.clone(),
        indexer_store.clone(),
        10,
        PRUNE_INTERVAL_DEFAULT,
        MAINNET_CANONICAL_THRESHOLD,
    )
    .unwrap();

    // add all blocks to the state
    state0.add_blocks(&mut block_parser).await.unwrap();

    // fresh state to sync events
    let mut state1 = IndexerState::new(
        &MAINNET_GENESIS_HASH.into(),
        genesis_ledger,
        indexer_store,
        10,
        PRUNE_INTERVAL_DEFAULT,
        MAINNET_CANONICAL_THRESHOLD,
    )
    .unwrap();

    // sync state1 from state0's db
    state1.sync_from_db().unwrap();

    // witness trees are functionally equal
    assert_eq!(state0.best_tip_block(), state1.best_tip_block());
    assert_eq!(state0.canonical_tip_block(), state1.canonical_tip_block());
    assert_eq!(state0.diffs_map, state1.diffs_map);

    remove_dir_all(db_path).unwrap();
}
