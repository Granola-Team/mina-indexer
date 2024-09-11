use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::{parser::BlockParser, BlockWithoutHeight},
    constants::*,
    ledger::genesis::{GenesisLedger, GenesisRoot},
    server::IndexerVersion,
    state::{IndexerState, IndexerStateConfig},
    store::IndexerStore,
};
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn test() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("event-sync")?;
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
        false,
    )?;

    // add all blocks to the state
    state.add_blocks(&mut block_parser).await?;

    // fresh state to sync events with no genesis events
    let config = IndexerStateConfig::new(
        genesis_ledger.into(),
        IndexerVersion::default(),
        indexer_store,
        MAINNET_CANONICAL_THRESHOLD,
        10,
        false,
    );
    let mut state_sync = IndexerState::new_without_genesis_events(config)?;

    // sync from state's event store
    state_sync.sync_from_db()?;

    // witness trees are functionally equal
    let best_tip: BlockWithoutHeight = state.best_tip_block().clone().into();
    let canonical_root: BlockWithoutHeight = state.canonical_root_block().clone().into();
    let best_tip_sync: BlockWithoutHeight = state_sync.best_tip_block().clone().into();
    let canonical_root_sync: BlockWithoutHeight = state_sync.canonical_root_block().clone().into();

    assert_eq!(best_tip, best_tip_sync);
    assert_eq!(canonical_root, canonical_root_sync);
    assert_eq!(state.blocks, state_sync.blocks);
    Ok(())
}
