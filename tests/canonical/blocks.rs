use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::{parser::BlockParser, BlockHash},
    canonical::store::CanonicityStore,
    state::{ledger::genesis::GenesisRoot, Canonicity, IndexerState},
    store::IndexerStore,
    MAINNET_CANONICAL_THRESHOLD, MAINNET_GENESIS_HASH, PRUNE_INTERVAL_DEFAULT,
};
use std::{fs::remove_dir_all, path::PathBuf, sync::Arc};

#[tokio::test]
async fn test() {
    let db_path = setup_new_db_dir("./test_canonical_blocks_store");
    let log_dir = PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");
    let mut block_parser = BlockParser::new_testing(&log_dir).unwrap();
    let indexer_store = Arc::new(IndexerStore::new(&db_path).unwrap());
    let genesis_contents = include_str!("../data/genesis_ledgers/mainnet.json");
    let genesis_ledger = serde_json::from_str::<GenesisRoot>(genesis_contents)
        .unwrap()
        .ledger;
    let mut state = IndexerState::new(
        BlockHash(MAINNET_GENESIS_HASH.to_string()),
        genesis_ledger,
        indexer_store,
        10,
        PRUNE_INTERVAL_DEFAULT,
        MAINNET_CANONICAL_THRESHOLD,
    )
    .unwrap();

    state.add_blocks(&mut block_parser).await.unwrap();

    println!("CANONICAL TIP: {:?}", state.canonical_tip_block());
    println!("BEST TIP:      {:?}", state.best_tip_block());
    println!("{state}");

    assert_eq!(block_parser.total_num_blocks, 20);

    let indexer_store = state.indexer_store.as_ref().unwrap();
    let max_canonical_height = indexer_store
        .get_max_canonical_blockchain_length()
        .unwrap()
        .unwrap();
    let canonical_hashes = vec![
        MAINNET_GENESIS_HASH.to_string(),
        "3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH".to_string(),
        "3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R".to_string(),
        "3NL9qBsNibXPm5Nh8cSg5CCqrbzX5VUVY9gJzAbg7EVCF3hfhazG".to_string(),
        "3NKQUoBfi9vkbuqtDJmSEYBQrcSo4GjwG8bPCiii4yqM8AxEQvtY".to_string(),
        "3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v".to_string(),
        "3NLGcwFVQF1p1PrZpusw2fZwBe5HKXGtrGy1Vc4aPkeBtT8nMNUc".to_string(),
        "3NLVZQz4FwFbvW4hejfyRpw5NyP8XvQjhj4wSsCjCKdHNBjwWsPG".to_string(),
        "3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw".to_string(),
        "3NKGgTk7en3347KH81yDra876GPAUSoSePrfVKPmwR1KHfMpvJC5".to_string(),
        "3NLMeYAFXxsmhSFtLHFxdtjGcfHTVFmBmBF8uTJvP4Ve5yEmxYeA".to_string(),
    ];

    assert_eq!(max_canonical_height, canonical_hashes.len() as u32);

    for n in 1..=max_canonical_height {
        let hash = &indexer_store
            .get_canonical_hash_at_height(n)
            .unwrap()
            .unwrap()
            .0;
        assert_eq!(hash, canonical_hashes.get((n - 1) as usize).unwrap());
    }

    for n in 2..=max_canonical_height {
        assert_eq!(
            Some(Canonicity::Canonical),
            state
                .get_block_status(&BlockHash(
                    canonical_hashes.get((n - 1) as usize).unwrap().to_string(),
                ))
                .unwrap()
        );
    }

    remove_dir_all(db_path).unwrap();
}
