use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::parser::BlockParser,
    constants::*,
    ledger::genesis::{GenesisLedger, GenesisRoot},
    server::IndexerVersion,
    state::{IndexerState, IndexerStateConfig},
    store::IndexerStore,
};
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn test() {
    let store_dir = setup_new_db_dir("event-replay").unwrap();
    let log_dir = PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");
    let mut block_parser = BlockParser::new_testing(&log_dir).unwrap();
    let indexer_store = Arc::new(IndexerStore::new(store_dir.path()).unwrap());
    let genesis_ledger =
        serde_json::from_str::<GenesisRoot>(GenesisLedger::MAINNET_V1_GENESIS_LEDGER_CONTENTS)
            .unwrap();
    let mut state = IndexerState::new(
        genesis_ledger.clone().into(),
        IndexerVersion::new_testing(),
        indexer_store.clone(),
        MAINNET_CANONICAL_THRESHOLD,
        10,
    )
    .unwrap();

    // add all blocks to the state
    state.add_blocks(&mut block_parser).unwrap();

    // fresh state to replay events on top of
    let config = IndexerStateConfig::new(
        genesis_ledger.into(),
        IndexerVersion::new_testing(),
        indexer_store,
        MAINNET_CANONICAL_THRESHOLD,
        10,
    );
    let mut new_state = IndexerState::new_without_genesis_events(config).unwrap();

    // replay events on the fresh state
    new_state.replay_events().unwrap();

    // witness trees match
    assert_eq!(state.best_tip_block(), new_state.best_tip_block());
    assert_eq!(
        state.canonical_root_block(),
        new_state.canonical_root_block()
    );
    assert_eq!(state.diffs_map, new_state.diffs_map);
}
