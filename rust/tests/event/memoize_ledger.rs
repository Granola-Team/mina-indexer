use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::{parser::BlockParser, BlockHash},
    event::{
        db::{DbEvent, DbLedgerEvent},
        store::EventStore,
        IndexerEvent,
    },
    ledger::{genesis::GenesisRoot, store::LedgerStore, LedgerHash},
    state::IndexerState,
    store::IndexerStore,
};
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn test() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("./test_memoize_ledger")?;
    let log_dir = PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");
    let mut block_parser = BlockParser::new_testing(&log_dir)?;
    let indexer_store = Arc::new(IndexerStore::new(store_dir.path())?);
    let genesis_contents = include_str!("../data/genesis_ledgers/mainnet.json");
    let genesis_ledger = serde_json::from_str::<GenesisRoot>(genesis_contents)?;
    let mut state = IndexerState::new(genesis_ledger.clone().into(), indexer_store.clone(), 10)?;

    // add all blocks & get store handle
    state.add_blocks(&mut block_parser)?;
    let network = "mainnet".to_string();
    let indexer_store = state.indexer_store.as_ref().unwrap();

    // memoize via state hash query
    // mainnet-7-3NLGcwFVQF1p1PrZpusw2fZwBe5HKXGtrGy1Vc4aPkeBtT8nMNUc.json
    let blockchain_length = 7;
    let state_hash = BlockHash("3NLGcwFVQF1p1PrZpusw2fZwBe5HKXGtrGy1Vc4aPkeBtT8nMNUc".into());
    let ledger_hash = LedgerHash("jwFtwfnhd2PDb15c23uVgNqjS3PNVWP4HpZzYSVGQAv64Y2bdV5".into());
    assert!(indexer_store
        .get_ledger_state_hash(&network, &state_hash, true)?
        .is_some());

    // don't memoize via state hash query
    // mainnet-6-3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v.json
    let blockchain_length_no = 6;
    let state_hash_no = BlockHash("3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v".into());
    let ledger_hash_no = LedgerHash("jxqrHaBcJzZAPW2rSa84chAxEHW7ot2GbqmRsWuNhwctZ8TFA2K".into());
    assert!(indexer_store
        .get_ledger_state_hash(&network, &state_hash_no, false)?
        .is_some());

    // check the event log for new ledger event
    let event_log = indexer_store.get_event_log()?;
    assert!(event_log.contains(&IndexerEvent::Db(DbEvent::Ledger(
        DbLedgerEvent::NewLedger {
            network: network.clone(),
            state_hash,
            ledger_hash,
            blockchain_length,
        }
    ))));
    // check the event log does not contain new ledger event
    assert!(!event_log.contains(&IndexerEvent::Db(DbEvent::Ledger(
        DbLedgerEvent::NewLedger {
            network: network.clone(),
            state_hash: state_hash_no,
            ledger_hash: ledger_hash_no,
            blockchain_length: blockchain_length_no,
        }
    ))));

    // memoize via height query
    // mainnet-4-3NL9qBsNibXPm5Nh8cSg5CCqrbzX5VUVY9gJzAbg7EVCF3hfhazG.json
    let blockchain_length = 4;
    let state_hash = BlockHash("3NL9qBsNibXPm5Nh8cSg5CCqrbzX5VUVY9gJzAbg7EVCF3hfhazG".into());
    let ledger_hash = LedgerHash("jxw3wNhAUhyVT4AK4dGxtn4Kpx6pvk3AXVoi2A6BAEQweyV8Uwe".into());
    assert!(indexer_store
        .get_ledger_at_height(&network, blockchain_length, true)?
        .is_some());

    // don't memoize via height query
    // mainnet-11-3NLMeYAFXxsmhSFtLHFxdtjGcfHTVFmBmBF8uTJvP4Ve5yEmxYeA.json
    let blockchain_length_no = 11;
    let state_hash_no = BlockHash("3NLMeYAFXxsmhSFtLHFxdtjGcfHTVFmBmBF8uTJvP4Ve5yEmxYeA".into());
    let ledger_hash_no = LedgerHash("jxZVWjsyuQkPVSj7ZbqC8PPx8FXzHQjxUYA3bhvdnQQZ15jn7mR".into());
    assert!(indexer_store
        .get_ledger_at_height(&network, blockchain_length_no, false)?
        .is_some());

    // check the event log for new ledger event
    let event_log = indexer_store.get_event_log()?;
    assert!(event_log.contains(&IndexerEvent::Db(DbEvent::Ledger(
        DbLedgerEvent::NewLedger {
            network: network.clone(),
            state_hash,
            ledger_hash,
            blockchain_length,
        }
    ))));
    // check the event log does not contain new ledger event
    assert!(!event_log.contains(&IndexerEvent::Db(DbEvent::Ledger(
        DbLedgerEvent::NewLedger {
            network,
            state_hash: state_hash_no,
            ledger_hash: ledger_hash_no,
            blockchain_length: blockchain_length_no,
        }
    ))));

    Ok(())
}
