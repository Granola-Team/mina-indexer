use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::{parser::BlockParser, BlockWithoutHeight},
    constants::{
        LEDGER_CADENCE, MAINNET_CANONICAL_THRESHOLD, MAINNET_GENESIS_HASH, PRUNE_INTERVAL_DEFAULT,
    },
    ledger::genesis::GenesisRoot,
    state::IndexerState,
    store::IndexerStore,
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
    let mut state = IndexerState::new(
        &MAINNET_GENESIS_HASH.into(),
        genesis_ledger.clone(),
        indexer_store.clone(),
        10,
        PRUNE_INTERVAL_DEFAULT,
        MAINNET_CANONICAL_THRESHOLD,
        LEDGER_CADENCE,
    )
    .unwrap();

    // add all blocks to the state
    state.add_blocks(&mut block_parser).unwrap();

    // fresh state to sync events with no genesis events
    let mut state_sync = IndexerState::new_without_genesis_events(
        &MAINNET_GENESIS_HASH.into(),
        genesis_ledger,
        indexer_store,
        10,
        PRUNE_INTERVAL_DEFAULT,
        MAINNET_CANONICAL_THRESHOLD,
        LEDGER_CADENCE,
    )
    .unwrap();

    // sync from state's event store
    state_sync.sync_from_db().unwrap();

    // witness trees are functionally equal
    let best_tip: BlockWithoutHeight = state.best_tip_block().clone().into();
    let canonical_tip: BlockWithoutHeight = state.canonical_tip_block().clone().into();
    let best_tip_sync: BlockWithoutHeight = state_sync.best_tip_block().clone().into();
    let canonical_tip_sync: BlockWithoutHeight = state_sync.canonical_tip_block().clone().into();

    assert_eq!(best_tip, best_tip_sync);
    assert_eq!(canonical_tip, canonical_tip_sync);

    for state_hash in state_sync.diffs_map.keys() {
        assert_eq!(
            state.diffs_map.get(state_hash),
            state_sync.diffs_map.get(state_hash)
        );
    }

    remove_dir_all(db_path).unwrap();
}
