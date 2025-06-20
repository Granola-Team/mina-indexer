use crate::helpers::{state::*, store::*};
use mina_indexer::{
    block::{parser::BlockParser, BlockWithoutHeight},
    constants::*,
    ledger::genesis::GenesisLedger,
    server::IndexerVersion,
    state::{IndexerState, IndexerStateConfig},
};
use std::path::PathBuf;

#[tokio::test]
async fn test() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("event-sync")?;
    let block_dir = PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");

    let mut block_parser = BlockParser::new_testing(&block_dir)?;
    let mut state = mainnet_genesis_state(store_dir.as_ref())?;

    // add all blocks to the state
    state.add_blocks(&mut block_parser).await?;

    let store = state.indexer_store.as_ref().unwrap();

    // fresh state to sync events with no genesis events
    let config = IndexerStateConfig::new(
        GenesisLedger::new_v1()?,
        IndexerVersion::default(),
        store.clone(),
        MAINNET_CANONICAL_THRESHOLD,
        10,
        false,
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

    // root & tip match
    assert_eq!(best_tip, best_tip_sync);
    assert_eq!(canonical_root, canonical_root_sync);

    // sync diffs contained in original diffs
    for state_hash in state_sync.diffs_map.keys() {
        assert_eq!(
            state.diffs_map.get(state_hash),
            state_sync.diffs_map.get(state_hash)
        );
    }

    // no dangling branches
    assert!(state.dangling_branches.is_empty());
    assert!(state_sync.dangling_branches.is_empty());
    Ok(())
}
