use crate::helpers::setup_new_db_dir;
use mina_indexer::{
    block::{parser::BlockParser, store::BlockStore, BlockHash},
    canonicity::{store::CanonicityStore, Canonicity},
    constants::*,
    ledger::genesis::{GenesisLedger, GenesisRoot},
    server::IndexerVersion,
    state::IndexerState,
    store::IndexerStore,
};
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn test() -> anyhow::Result<()> {
    let store_dir = setup_new_db_dir("canonicity-blocks")?;
    let log_dir = PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");
    let mut block_parser = BlockParser::new_testing(&log_dir)?;
    let indexer_store = Arc::new(IndexerStore::new(store_dir.path())?);
    let genesis_ledger =
        serde_json::from_str::<GenesisRoot>(GenesisLedger::MAINNET_V1_GENESIS_LEDGER_CONTENTS)?;
    let mut state = IndexerState::new(
        genesis_ledger.into(),
        IndexerVersion::default(),
        indexer_store.clone(),
        MAINNET_CANONICAL_THRESHOLD,
        MAINNET_TRANSITION_FRONTIER_K,
    )?;

    state.add_blocks(&mut block_parser).await?;

    println!("CANONICAL ROOT: {:?}", state.canonical_root_block());
    println!("BEST TIP:       {:?}", state.best_tip_block());
    println!("{state}");

    assert_eq!(block_parser.total_num_blocks, 20);

    let indexer_store = state.indexer_store.as_ref().unwrap();
    let best_block_height = indexer_store.get_best_block_height()?.unwrap();
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
        "3NKkJDmNZGYdKVDDJkkamGdvNzASia2SXxKpu18imps7KqbNXENY".to_string(),
        "3NKXzc1hAE1bK9BSkJUhBBSznMhwW3ZxUTgdoLoqzW6SvqVFcAw5".to_string(),
        "3NKDTKbWye6GcdjRu28sSSUgwkNDZXZJvsVZpXAR4YeawhYLqjtE".to_string(),
        "3NKkVW47d5Zxi7zvKufBrbiAvLzyKnFgsnN9vgCw65sffvHpv63M".to_string(),
        "3NL1sy75LXQScPZda2ywNmdVPiJDnYFe5wV7YLzyRcPVgmDkemW9".to_string(),
        "3NKDWsSnHUHN6iakRuBY4LcNou8ToQ3jHpMWkyp6gposjjXC6XUu".to_string(),
        "3NLZhhUTMGiWe9UYxY8aYHvRVSoKJTHgKJvopBdC2RA9KisGfPuo".to_string(),
        "3NLEu5K5pmEH1CSKZJd94eJatDTM3djoeJTVE3RkcNztJ4z63bM6".to_string(),
        "3NLPpt5SyVnD1U5uJAqR3DL1Cqj5dG26SuWutRQ6AQpbQtQUWSYA".to_string(),
        "3NKZ6DTHiMtuaeP3tJq2xe4uujVRnGT9FX1rBiZY521uNToSppUZ".to_string(),
    ];

    assert_eq!(best_block_height, canonical_hashes.len() as u32);

    for n in 1..=best_block_height {
        let hash = &indexer_store
            .get_canonical_hash_at_height(n)
            .unwrap()
            .unwrap()
            .0;
        assert_eq!(hash, canonical_hashes.get((n - 1) as usize).unwrap());
    }

    for n in 2..=best_block_height {
        assert_eq!(
            Some(Canonicity::Canonical),
            state
                .get_block_status(&BlockHash(
                    canonical_hashes.get((n - 1) as usize).unwrap().to_string(),
                ))
                .unwrap()
        );
    }
    Ok(())
}
