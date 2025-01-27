use crate::helpers::{state::*, store::*};
use mina_indexer::{
    block::parser::BlockParser,
    constants::*,
    ledger::genesis::GenesisLedger,
    server::IndexerVersion,
    state::{IndexerState, IndexerStateConfig},
};
use std::path::PathBuf;

#[tokio::test]
async fn test() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("event-replay")?;
    let block_dir = PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");

    let mut state = mainnet_genesis_state(store_dir.as_ref())?;
    let mut block_parser = BlockParser::new_testing(&block_dir)?;

    // add all blocks to the state
    state.add_blocks(&mut block_parser).await?;

    let store = state.indexer_store.as_ref().unwrap();

    // fresh state to replay events on top of
    let config = IndexerStateConfig::new(
        GenesisLedger::new_v1()?,
        IndexerVersion::default(),
        store.clone(),
        MAINNET_CANONICAL_THRESHOLD,
        10,
        false,
    );
    let mut new_state = IndexerState::new_without_genesis_events(config)?;

    // replay events on the fresh state
    new_state.replay_events(&state)?;

    /////////////////////////
    // witness trees match //
    /////////////////////////

    // best blocks match
    assert_eq!(state.best_tip_block(), new_state.best_tip_block());

    // root blocks match
    assert_eq!(
        state.canonical_root_block(),
        new_state.canonical_root_block()
    );

    // diffs maps match
    assert_eq!(state.diffs_map, new_state.diffs_map);

    Ok(())
}
