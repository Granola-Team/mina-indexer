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
async fn test() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("event-replay")?;
    let log_dir = PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");
    let mut block_parser = BlockParser::new_testing(&log_dir)?;
    let indexer_store = Arc::new(IndexerStore::new(store_dir.path())?);
    let genesis_ledger =
        serde_json::from_str::<GenesisRoot>(GenesisLedger::MAINNET_V1_GENESIS_LEDGER_CONTENTS)?;
    let mut state = IndexerState::new(
        genesis_ledger.clone().into(),
        IndexerVersion::default(),
        indexer_store.clone(),
        MAINNET_CANONICAL_THRESHOLD,
        10,
    )?;

    // add all blocks to the state
    state.add_blocks(&mut block_parser).await?;

    // fresh state to replay events on top of
    let config = IndexerStateConfig::new(
        genesis_ledger.into(),
        IndexerVersion::default(),
        indexer_store,
        MAINNET_CANONICAL_THRESHOLD,
        10,
    );
    let mut new_state = IndexerState::new_without_genesis_events(config)?;

    // replay events on the fresh state
    new_state.replay_events(&state)?;

    // witness trees match
    assert_eq!(state.best_tip_block(), new_state.best_tip_block());
    assert_eq!(
        state.canonical_root_block(),
        new_state.canonical_root_block()
    );
    assert_eq!(state.diffs_map, new_state.diffs_map);
    Ok(())
}
