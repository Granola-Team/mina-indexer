use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::parser::BlockParser, ledger::genesis::GenesisRoot, state::IndexerState,
    store::IndexerStore, MAINNET_CANONICAL_THRESHOLD, MAINNET_GENESIS_HASH, PRUNE_INTERVAL_DEFAULT,
};
use std::{fs::remove_dir_all, path::PathBuf, sync::Arc};

#[tokio::test]
async fn test() {
    let db_path = setup_new_db_dir("./test-event-replay");
    let log_dir = PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");
    let mut block_parser = BlockParser::new_testing(&log_dir).unwrap();
    let indexer_store = Arc::new(IndexerStore::new(&db_path).unwrap());
    let genesis_contents = include_str!("../data/genesis_ledgers/mainnet.json");
    let genesis_ledger = serde_json::from_str::<GenesisRoot>(genesis_contents)
        .unwrap()
        .ledger;
    let mut state = IndexerState::new(
        &MAINNET_GENESIS_HASH.into(),
        genesis_ledger,
        indexer_store.clone(),
        10,
        PRUNE_INTERVAL_DEFAULT,
        MAINNET_CANONICAL_THRESHOLD,
    )
    .unwrap();

    // add all blocks to the state
    state.add_blocks(&mut block_parser).await.unwrap();

    // fresh state to replay events on top of
    let mut new_state = IndexerState::new_without_genesis_events(
        &MAINNET_GENESIS_HASH.into(),
        indexer_store,
        10,
        PRUNE_INTERVAL_DEFAULT,
        MAINNET_CANONICAL_THRESHOLD,
    )
    .unwrap();

    // replay events on the fresh state
    new_state.replay_events().unwrap();

    // witness trees match
    assert_eq!(state.best_tip_block(), new_state.best_tip_block());
    assert_eq!(state.canonical_tip_block(), new_state.canonical_tip_block());
    assert_eq!(state.diffs_map, new_state.diffs_map);

    remove_dir_all(db_path).unwrap();
}
