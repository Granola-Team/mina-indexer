use crate::helpers::{hardfork_genesis_state, setup_new_db_dir};
use mina_indexer::{
    block::{genesis::GenesisBlock, store::BlockStore},
    constants::*,
    ledger::genesis::parse_file,
    server::IndexerVersion,
    state::IndexerState,
    store::IndexerStore,
};
use std::{path::PathBuf, sync::Arc};

#[test]
fn genesis_v1() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("block-store-genesis-v1")?;
    let indexer_store = Arc::new(IndexerStore::new(store_dir.path())?);
    let genesis_ledger_path = &PathBuf::from("./data/genesis_ledgers/mainnet.json");
    let genesis_root = parse_file(genesis_ledger_path)?;

    let state = IndexerState::new(
        genesis_root.into(),
        IndexerVersion::default(),
        indexer_store,
        MAINNET_CANONICAL_THRESHOLD,
        MAINNET_TRANSITION_FRONTIER_K,
        false,
        false,
    )?;

    // check v1 genesis block is in the block store
    assert_eq!(
        state
            .indexer_store
            .unwrap()
            .get_block(&MAINNET_GENESIS_HASH.into())
            .unwrap()
            .map(|b| b.0),
        Some(GenesisBlock::new_v1().unwrap().to_precomputed())
    );

    Ok(())
}

#[test]
fn genesis_v2_add() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("block-store-genesis-v2-add")?;
    let indexer_store = Arc::new(IndexerStore::new(store_dir.path())?);
    let genesis_ledger_path = &PathBuf::from("./data/genesis_ledgers/mainnet.json");
    let genesis_root = parse_file(genesis_ledger_path)?;

    let mut state = IndexerState::new(
        genesis_root.into(),
        IndexerVersion::default(),
        indexer_store,
        MAINNET_CANONICAL_THRESHOLD,
        MAINNET_TRANSITION_FRONTIER_K,
        false,
        false,
    )?;

    let GenesisBlock(block, block_bytes) = GenesisBlock::new_v2()?;

    // add v2 genesis block
    assert!(state.block_pipeline(&block, block_bytes)?);

    // check v2 genesis block is in the block store
    assert_eq!(
        state
            .indexer_store
            .unwrap()
            .get_block(&HARDFORK_GENESIS_HASH.into())
            .unwrap()
            .map(|b| b.0),
        Some(block)
    );

    Ok(())
}

#[test]
fn genesis_v2_start() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("block-store-genesis-v2-start")?;
    let state = hardfork_genesis_state(store_dir.path())?;

    // check v2 genesis block is in the block store
    assert_eq!(
        state
            .indexer_store
            .unwrap()
            .get_block(&HARDFORK_GENESIS_HASH.into())
            .unwrap()
            .map(|b| b.0),
        Some(GenesisBlock::new_v2().unwrap().to_precomputed())
    );

    Ok(())
}
