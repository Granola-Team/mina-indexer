use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::parser::BlockParser,
    event::{db::*, store::EventStore, IndexerEvent},
    ledger::genesis::GenesisRoot,
    state::IndexerState,
    store::IndexerStore,
};
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn test() {
    let store_dir = setup_new_db_dir("event-witness-tree").unwrap();
    let log_dir = PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");
    let mut block_parser = BlockParser::new_testing(&log_dir).unwrap();
    let indexer_store = Arc::new(IndexerStore::new(store_dir.path()).unwrap());
    let genesis_contents = include_str!("../data/genesis_ledgers/mainnet.json");
    let genesis_root = serde_json::from_str::<GenesisRoot>(genesis_contents).unwrap();
    let mut state =
        IndexerState::new(genesis_root.clone().into(), indexer_store.clone(), 10).unwrap();

    // add all blocks to the state
    state.add_blocks(&mut block_parser).await.unwrap();

    let event_log = indexer_store.get_event_log().unwrap();

    // last update best tip
    let last_best_tip = event_log
        .iter()
        .filter_map(|event| match event {
            IndexerEvent::Db(DbEvent::Block(DbBlockEvent::NewBestTip {
                state_hash,
                network: _,
                blockchain_length: _,
            })) => Some(state_hash.clone()),
            _ => None,
        })
        .last()
        .unwrap();
    assert_eq!(
        last_best_tip.0,
        "3NKZ6DTHiMtuaeP3tJq2xe4uujVRnGT9FX1rBiZY521uNToSppUZ".to_string()
    );

    // canonical events
    let canonical_events: Vec<(u32, &str)> = event_log
        .iter()
        .filter_map(|event| match event {
            IndexerEvent::Db(DbEvent::Canonicity(DbCanonicityEvent::NewCanonicalBlock {
                network: _,
                state_hash,
                blockchain_length,
            })) => Some((*blockchain_length, &state_hash.0[..])),
            _ => None,
        })
        .collect();
    assert_eq!(
        canonical_events,
        vec![
            (1, "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ"),
            (2, "3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH"),
            (3, "3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R"),
            (4, "3NL9qBsNibXPm5Nh8cSg5CCqrbzX5VUVY9gJzAbg7EVCF3hfhazG"),
            (5, "3NKQUoBfi9vkbuqtDJmSEYBQrcSo4GjwG8bPCiii4yqM8AxEQvtY"),
            (6, "3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v"),
            (7, "3NLGcwFVQF1p1PrZpusw2fZwBe5HKXGtrGy1Vc4aPkeBtT8nMNUc"),
            (8, "3NLVZQz4FwFbvW4hejfyRpw5NyP8XvQjhj4wSsCjCKdHNBjwWsPG"),
            (9, "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw"),
            (10, "3NKGgTk7en3347KH81yDra876GPAUSoSePrfVKPmwR1KHfMpvJC5"),
            (11, "3NLMeYAFXxsmhSFtLHFxdtjGcfHTVFmBmBF8uTJvP4Ve5yEmxYeA"),
        ]
    );
}
