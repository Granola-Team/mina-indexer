use mina_indexer::{
    block::{genesis::GenesisBlock, store::BlockStore},
    state::ledger::genesis::parse_file,
    state::IndexerState,
    store::IndexerStore,
    CANONICAL_UPDATE_THRESHOLD, MAINNET_GENESIS_HASH, MAINNET_TRANSITION_FRONTIER_K,
    PRUNE_INTERVAL_DEFAULT,
};
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn block_added() {
    let mut store_dir = std::env::temp_dir();
    store_dir.push("./genesis-block-test");

    let indexer_store = Arc::new(IndexerStore::new(&store_dir).unwrap());
    let genesis_ledger_path = &PathBuf::from("./tests/data/genesis_ledgers/mainnet.json");
    let genesis_root = parse_file(genesis_ledger_path).unwrap();

    let indexer = IndexerState::new(
        &MAINNET_GENESIS_HASH.into(),
        genesis_root.ledger,
        indexer_store,
        MAINNET_TRANSITION_FRONTIER_K,
        PRUNE_INTERVAL_DEFAULT,
        CANONICAL_UPDATE_THRESHOLD,
    )
    .unwrap();

    assert_eq!(
        indexer
            .indexer_store
            .unwrap()
            .get_block(&MAINNET_GENESIS_HASH.into())
            .unwrap(),
        Some(GenesisBlock::new().unwrap().into())
    )
}
