use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::{parser::BlockParser, BlockWithoutHeight},
    ledger::genesis::GenesisRoot,
    state::{IndexerState, IndexerStateConfig},
    store::IndexerStore,
};
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn test() {
    let store_dir = setup_new_db_dir("event-sync").unwrap();
    let log_dir = PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");
    let mut block_parser = BlockParser::new_testing(&log_dir).unwrap();
    let indexer_store = Arc::new(IndexerStore::new(store_dir.path()).unwrap());
    let genesis_contents = include_str!("../data/genesis_ledgers/mainnet.json");
    let genesis_root = serde_json::from_str::<GenesisRoot>(genesis_contents).unwrap();
    let mut state =
        IndexerState::new(genesis_root.clone().into(), indexer_store.clone(), 10).unwrap();

    // add all blocks to the state
    state.add_blocks(&mut block_parser).await.unwrap();

    // fresh state to sync events with no genesis events
    let config = IndexerStateConfig::new(genesis_root.into(), indexer_store, 10);
    let mut state_sync = IndexerState::new_without_genesis_events(config).unwrap();

    // sync from state's event store
    state_sync.sync_from_db().unwrap();

    // witness trees are functionally equal
    let best_tip: BlockWithoutHeight = state.best_tip_block().clone().into();
    let canonical_root: BlockWithoutHeight = state.canonical_root_block().clone().into();
    let best_tip_sync: BlockWithoutHeight = state_sync.best_tip_block().clone().into();
    let canonical_root_sync: BlockWithoutHeight = state_sync.canonical_root_block().clone().into();

    assert_eq!(best_tip, best_tip_sync);
    assert_eq!(canonical_root, canonical_root_sync);

    for state_hash in state_sync.diffs_map.keys() {
        assert_eq!(
            state.diffs_map.get(state_hash),
            state_sync.diffs_map.get(state_hash)
        );
    }
}
