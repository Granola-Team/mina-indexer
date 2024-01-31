use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::{genesis::GenesisBlock, store::BlockStore},
    constants::{
        CANONICAL_UPDATE_THRESHOLD, LEDGER_CADENCE, MAINNET_GENESIS_HASH,
        MAINNET_TRANSITION_FRONTIER_K, PRUNE_INTERVAL_DEFAULT,
    },
    ledger::genesis::parse_file,
    state::IndexerState,
    store::IndexerStore,
};
use std::{path::PathBuf, sync::Arc};

#[test]
fn block_added() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("block-store-genesis")?;
    let indexer_store = Arc::new(IndexerStore::new(store_dir.path())?);
    let genesis_ledger_path = &PathBuf::from("./tests/data/genesis_ledgers/mainnet.json");
    let genesis_root = parse_file(genesis_ledger_path)?;

    let indexer = IndexerState::new(
        &MAINNET_GENESIS_HASH.into(),
        genesis_root.into(),
        indexer_store,
        MAINNET_TRANSITION_FRONTIER_K,
        PRUNE_INTERVAL_DEFAULT,
        CANONICAL_UPDATE_THRESHOLD,
        LEDGER_CADENCE,
    )?;

    assert_eq!(
        indexer
            .indexer_store
            .unwrap()
            .get_block(&MAINNET_GENESIS_HASH.into())
            .unwrap(),
        Some(GenesisBlock::new().unwrap().into())
    );
    Ok(())
}
