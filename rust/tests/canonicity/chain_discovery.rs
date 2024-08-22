use glob::glob;
use mina_indexer::{
    block::{
        parser::BlockParser,
        precomputed::{PcbVersion, PrecomputedBlock},
    },
    canonicity::canonical_chain_discovery::discovery,
    constants::*,
};
use std::path::PathBuf;

#[tokio::test]
async fn gaps() -> anyhow::Result<()> {
    let blocks_dir = PathBuf::from("./tests/data/canonical_chain_discovery/gaps");
    let mut block_parser = BlockParser::new_with_canonical_chain_discovery(
        &blocks_dir,
        PcbVersion::V1,
        MAINNET_CANONICAL_THRESHOLD,
        BLOCK_REPORTING_FREQ_NUM,
    )
    .await?;

    while let Some((block, _)) = block_parser.next_block().await? {
        let block: PrecomputedBlock = block.into();
        println!("{}", block.summary());
    }

    // only length 2 is known to be canonical
    assert_eq!(block_parser.total_num_blocks, 26);
    assert_eq!(block_parser.num_deep_canonical_blocks, 1);
    Ok(())
}

#[tokio::test]
async fn contiguous() -> anyhow::Result<()> {
    let blocks_dir = PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");
    let mut block_parser = BlockParser::new_with_canonical_chain_discovery(
        &blocks_dir,
        PcbVersion::V1,
        MAINNET_CANONICAL_THRESHOLD,
        BLOCK_REPORTING_FREQ_NUM,
    )
    .await?;

    while let Some((block, _)) = block_parser.next_block().await? {
        let block: PrecomputedBlock = block.into();
        println!("{}", block.summary());
    }

    // lengths 2..11 are known to be canonical
    assert_eq!(block_parser.total_num_blocks, 20);
    assert_eq!(block_parser.num_deep_canonical_blocks, 10);
    Ok(())
}

#[tokio::test]
async fn missing_parent() -> anyhow::Result<()> {
    let blocks_dir = PathBuf::from("./tests/data/canonical_chain_discovery/missing_parent");
    let mut block_parser = BlockParser::new_with_canonical_chain_discovery(
        &blocks_dir,
        PcbVersion::V1,
        MAINNET_CANONICAL_THRESHOLD,
        BLOCK_REPORTING_FREQ_NUM,
    )
    .await?;

    while let Some((block, _)) = block_parser.next_block().await? {
        let block: PrecomputedBlock = block.into();
        println!("{}", block.summary());
    }

    // mainnet-105500-3NKvv2iBAPhZ8SRCxQEuGTgqTYuFXd2WVANXW6pcsR8pdzLuUj7C.json
    // which should be present in the canonical chain, is absent from the
    // collection. Therefore we can't determine the canonical root from either
    // block with length 105501. Thus, the highest canonical root we can find is
    // mainnet-105490-3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC.json
    // and the best canonical consists of blocks with lengths 105487, 105488,
    // 105489, 105490.
    assert_eq!(
        block_parser.total_num_blocks,
        glob(&format!("{}/*-*-*.json", blocks_dir.display()))
            .unwrap()
            .count() as u32
    );
    assert_eq!(block_parser.num_deep_canonical_blocks, 4);
    Ok(())
}

#[tokio::test]
async fn one_block() -> anyhow::Result<()> {
    let blocks_dir = PathBuf::from("./tests/data/canonical_chain_discovery/one_block");
    let block_parser = BlockParser::new_with_canonical_chain_discovery(
        &blocks_dir,
        PcbVersion::V1,
        MAINNET_CANONICAL_THRESHOLD,
        BLOCK_REPORTING_FREQ_NUM,
    )
    .await?;

    assert_eq!(block_parser.total_num_blocks, 1);
    assert_eq!(block_parser.num_deep_canonical_blocks, 0);
    Ok(())
}

#[tokio::test]
async fn canonical_threshold() -> anyhow::Result<()> {
    let canonical_threshold = 2;
    let blocks_dir = PathBuf::from("./tests/data/canonical_chain_discovery/contiguous");
    let mut block_parser = BlockParser::new_with_canonical_chain_discovery(
        &blocks_dir,
        PcbVersion::V1,
        canonical_threshold,
        BLOCK_REPORTING_FREQ_NUM,
    )
    .await?;

    while let Some((block, _)) = block_parser.next_block().await? {
        let block: PrecomputedBlock = block.into();
        println!("{}", block.summary());
    }

    // lengths 2..19 are known to be canonical
    assert_eq!(
        block_parser.num_deep_canonical_blocks,
        block_parser.total_num_blocks - canonical_threshold
    );
    assert_eq!(block_parser.total_num_blocks, 20);
    Ok(())
}

#[tokio::test]
async fn discovery_algorithm() -> anyhow::Result<()> {
    let blocks_dir_str = "tests/data/canonical_chain_discovery/contiguous/";
    let blocks_dir = PathBuf::from(blocks_dir_str);
    let pattern = format!("{}/*-*-*.json", blocks_dir.display());
    let paths: Vec<PathBuf> = glob(&pattern)?.filter_map(|x| x.ok()).collect();
    let (canonical_paths, recent_paths, orphaned_paths) = discovery(
        MAINNET_CANONICAL_THRESHOLD,
        BLOCK_REPORTING_FREQ_NUM,
        paths.iter().collect(),
    )?;

    assert_eq!(canonical_paths.len(), 10);
    assert_eq!(
        canonical_paths
            .iter()
            .map(|p| p.to_str().unwrap())
            .map(|p| p.strip_prefix(blocks_dir_str).unwrap())
            .collect::<Vec<_>>(),
        vec![
            "mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json",
            "mainnet-3-3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R.json",
            "mainnet-4-3NL9qBsNibXPm5Nh8cSg5CCqrbzX5VUVY9gJzAbg7EVCF3hfhazG.json",
            "mainnet-5-3NKQUoBfi9vkbuqtDJmSEYBQrcSo4GjwG8bPCiii4yqM8AxEQvtY.json",
            "mainnet-6-3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v.json",
            "mainnet-7-3NLGcwFVQF1p1PrZpusw2fZwBe5HKXGtrGy1Vc4aPkeBtT8nMNUc.json",
            "mainnet-8-3NLVZQz4FwFbvW4hejfyRpw5NyP8XvQjhj4wSsCjCKdHNBjwWsPG.json",
            "mainnet-9-3NKknQGpDQu6Afe1VYuHYbEfnjbHT3xGZaFCd8sueL8CoJkx5kPw.json",
            "mainnet-10-3NKGgTk7en3347KH81yDra876GPAUSoSePrfVKPmwR1KHfMpvJC5.json",
            "mainnet-11-3NLMeYAFXxsmhSFtLHFxdtjGcfHTVFmBmBF8uTJvP4Ve5yEmxYeA.json",
        ]
    );

    assert_eq!(recent_paths.len(), 10);
    assert_eq!(
        recent_paths
            .iter()
            .map(|p| p.to_str().unwrap())
            .map(|p| p.strip_prefix(blocks_dir_str).unwrap())
            .collect::<Vec<_>>(),
        vec![
            "mainnet-12-3NKkJDmNZGYdKVDDJkkamGdvNzASia2SXxKpu18imps7KqbNXENY.json",
            "mainnet-13-3NKXzc1hAE1bK9BSkJUhBBSznMhwW3ZxUTgdoLoqzW6SvqVFcAw5.json",
            "mainnet-14-3NKDTKbWye6GcdjRu28sSSUgwkNDZXZJvsVZpXAR4YeawhYLqjtE.json",
            "mainnet-15-3NKkVW47d5Zxi7zvKufBrbiAvLzyKnFgsnN9vgCw65sffvHpv63M.json",
            "mainnet-16-3NL1sy75LXQScPZda2ywNmdVPiJDnYFe5wV7YLzyRcPVgmDkemW9.json",
            "mainnet-17-3NKDWsSnHUHN6iakRuBY4LcNou8ToQ3jHpMWkyp6gposjjXC6XUu.json",
            "mainnet-18-3NLZhhUTMGiWe9UYxY8aYHvRVSoKJTHgKJvopBdC2RA9KisGfPuo.json",
            "mainnet-19-3NLEu5K5pmEH1CSKZJd94eJatDTM3djoeJTVE3RkcNztJ4z63bM6.json",
            "mainnet-20-3NLPpt5SyVnD1U5uJAqR3DL1Cqj5dG26SuWutRQ6AQpbQtQUWSYA.json",
            "mainnet-21-3NKZ6DTHiMtuaeP3tJq2xe4uujVRnGT9FX1rBiZY521uNToSppUZ.json",
        ]
    );

    assert_eq!(orphaned_paths.len(), 0);

    Ok(())
}
