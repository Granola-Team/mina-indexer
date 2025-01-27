use crate::helpers::{state::*, store::*};
use mina_indexer::{
    block::parser::BlockParser,
    event::{db::*, store::EventStore, IndexerEvent},
};
use std::path::PathBuf;

#[tokio::test]
async fn test() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("event-witness-tree")?;
    let block_dir = PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");

    let mut block_parser = BlockParser::new_testing(&block_dir)?;
    let mut state = mainnet_genesis_state(store_dir.as_ref())?;

    // add all blocks to the state
    state.add_blocks(&mut block_parser).await?;

    let store = state.indexer_store.as_ref().unwrap();

    // last update best tip
    let event_log = store.get_event_log()?;
    let last_best_tip = event_log
        .iter()
        .filter_map(|event| match event {
            IndexerEvent::Db(DbEvent::Block(DbBlockEvent::NewBestTip {
                state_hash,
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
    Ok(())
}
