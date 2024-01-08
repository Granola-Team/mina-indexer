use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::{parser::BlockParser, store::BlockStore, BlockHash},
    canonical::store::CanonicityStore,
    event::{state::StateEvent, store::EventStore},
    state::{ledger::genesis::GenesisRoot, IndexerState},
    store::IndexerStore,
    MAINNET_CANONICAL_THRESHOLD, MAINNET_GENESIS_HASH, PRUNE_INTERVAL_DEFAULT,
};
use std::{fs::remove_dir_all, path::PathBuf, sync::Arc};

#[tokio::test]
async fn test() {
    let log_dir = PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");

    let db_path0 = setup_new_db_dir("./test-event-log-store0");
    let mut block_parser0 = BlockParser::new(&log_dir, MAINNET_CANONICAL_THRESHOLD).unwrap();

    let db_path1 = setup_new_db_dir("./test-event-log-store1");
    let mut block_parser1 = BlockParser::new(&log_dir, MAINNET_CANONICAL_THRESHOLD).unwrap();

    let indexer_store0 = Arc::new(IndexerStore::new(&db_path0).unwrap());
    let indexer_store1 = Arc::new(IndexerStore::new(&db_path1).unwrap());

    let genesis_contents = include_str!("../data/genesis_ledgers/mainnet.json");
    let genesis_ledger = serde_json::from_str::<GenesisRoot>(genesis_contents)
        .unwrap()
        .ledger;

    let mut state0 = IndexerState::new(
        BlockHash(MAINNET_GENESIS_HASH.to_string()),
        genesis_ledger.clone(),
        indexer_store0,
        10,
        PRUNE_INTERVAL_DEFAULT,
        MAINNET_CANONICAL_THRESHOLD,
    )
    .unwrap();
    let mut state1 = IndexerState::new(
        BlockHash(MAINNET_GENESIS_HASH.to_string()),
        genesis_ledger,
        indexer_store1,
        10,
        PRUNE_INTERVAL_DEFAULT,
        MAINNET_CANONICAL_THRESHOLD,
    )
    .unwrap();

    // add parser0 blocks to state0
    state0.add_blocks(&mut block_parser0).await.unwrap();

    // add parser1 blocks to state1
    // - add block to db
    // - add block to witness tree
    // - update canonicities
    while let Some(block) = block_parser1.next_block().unwrap() {
        if let Some(db_event) = state1
            .indexer_store
            .as_ref()
            .map(|store| store.add_block(&block).unwrap())
        {
            let new_canonical_blocks = if db_event.is_new_block_event() {
                let (_, StateEvent::UpdateCanonicalChain(blocks)) =
                    state1.add_block(&block).unwrap();
                blocks
            } else {
                vec![]
            };

            new_canonical_blocks.iter().for_each(|block| {
                if let Some(store) = state1.indexer_store.as_ref() {
                    store
                        .add_canonical_block(block.blockchain_length, &block.state_hash)
                        .unwrap()
                }
            });
            state1.add_block(&block).unwrap();
        }
    }

    // check event logs match
    let event_log0 = if let Some(store0) = state0.indexer_store {
        store0.get_event_log().unwrap()
    } else {
        vec![]
    };
    let event_log1 = if let Some(store1) = state1.indexer_store {
        store1.get_event_log().unwrap()
    } else {
        vec![]
    };

    println!("----- Event log 0 -----");
    println!("{:?}", event_log0);
    println!("----- Event log 1 -----");
    println!("{:?}", event_log1);

    assert_eq!(event_log0, event_log1);

    remove_dir_all(db_path0).unwrap();
    remove_dir_all(db_path1).unwrap();
}
