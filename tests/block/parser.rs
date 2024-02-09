use mina_indexer::block::parser::BlockParser;
use std::path::PathBuf;
use tokio::time::Instant;

#[tokio::test]
async fn representative_benches() {
    let start = Instant::now();
    let sample_dir0 = PathBuf::from("./tests/data/non_sequential_blocks");
    let mut block_parser0 = BlockParser::new_testing(&sample_dir0).unwrap();
    let mut logs_processed = 0;

    while let Some(precomputed_block) = block_parser0
        .next_block()
        .expect("IO Error on block_parser")
    {
        logs_processed += 1;
        dbg!(precomputed_block.state_hash);
    }

    println!("./tests/data/non_sequential_blocks");
    println!("Parse {logs_processed} logs: {:?}\n", start.elapsed());

    assert_eq!(logs_processed, block_parser0.total_num_blocks);

    let start = Instant::now();
    let sample_dir1 = PathBuf::from("./tests/data/sequential_blocks");
    let mut block_parser1 = BlockParser::new_testing(&sample_dir1).unwrap();

    logs_processed = 0;
    while let Some(precomputed_block) = block_parser1
        .next_block()
        .expect("IO Error on block_parser")
    {
        logs_processed += 1;
        dbg!(precomputed_block.state_hash);
    }

    println!("./tests/data/sequential_blocks");
    println!("Parse {logs_processed} logs: {:?}\n", start.elapsed());

    assert_eq!(logs_processed, block_parser1.total_num_blocks);
}

#[tokio::test]
async fn get_global_slot_since_genesis() {
    let log_dir = PathBuf::from("./tests/data/sequential_blocks");
    let mut block_parser = BlockParser::new_testing(&log_dir).unwrap();

    // block = mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json
    let block = block_parser
        .get_precomputed_block("3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT")
        .await
        .unwrap();
    assert_eq!(
        block.state_hash,
        "3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT".to_owned()
    );
    assert_eq!(block.global_slot_since_genesis(), 155140);
}

#[tokio::test]
async fn orphaned_blocks() {
    use mina_indexer::{block::parser::BlockParserPaths, constants::*};

    let log_dir = PathBuf::from("./tests/data/sequential_blocks");
    let block_parser = BlockParser::new_with_canonical_chain_discovery(
        &log_dir,
        MAINNET_CANONICAL_THRESHOLD,
        MAINNET_TRANSITION_FRONTIER_K,
    )
    .unwrap();

    assert_eq!(
        block_parser.paths(),
        BlockParserPaths {
            canonical_paths: vec![
                "tests/data/sequential_blocks/mainnet-105489-3NK4huLvUDiL4XuCUcyrWCKynmvhqfKsx5h2MfBXVVUq2Qwzi5uT.json".into(),
                "tests/data/sequential_blocks/mainnet-105490-3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC.json".into(),
                "tests/data/sequential_blocks/mainnet-105491-3NKizDx3nnhXha2WqHDNUvJk9jW7GsonsEGYs26tCPW2Wow1ZoR3.json".into(),
            ],
            successive_paths: vec![
                "tests/data/sequential_blocks/mainnet-105492-3NKAqzELKDp2BbdKKwdRWEoMNehyMrxJGCoGCyH1t1PyyH7VQMgk.json".into(),
                "tests/data/sequential_blocks/mainnet-105492-3NKTUzjMZ8GD89XKD4qhnKZVXEfUSRGjHTYncZVQTxipZA9mnKZu.json".into(),
                "tests/data/sequential_blocks/mainnet-105492-3NKsUS3TtwvXsfFFnRAJ8US8wPLKKaRDTnbv4vzrwCDkb8HNaMWN.json".into(),
                "tests/data/sequential_blocks/mainnet-105492-3NKt8qae6VMefUXGdprN1Nve78zCQr9FFaMyRfQbj8Mza1FKcXEQ.json".into(),
                "tests/data/sequential_blocks/mainnet-105493-3NKakum3B2Tigw9TSsxwvXvV3x8L2LvrJ3yXFLEAJDMZu2vkn7db.json".into(),
                "tests/data/sequential_blocks/mainnet-105494-3NKXsaznJ6WdyA4PHfXxn25RzVanzQsNMZrxjidbhoBug8R4LZDy.json".into(),
                "tests/data/sequential_blocks/mainnet-105494-3NKqd3XGqkLmZVmPC3iG6AnrwQoZdBKdmYTzEJT3vwwnn2H1Z4ww.json".into(),
                "tests/data/sequential_blocks/mainnet-105494-3NLVgiopzZW9toJV4wCkggsCZJsQ3irSL5G5KfNN5CSrPTa3evpv.json".into(),
                "tests/data/sequential_blocks/mainnet-105495-3NKmDYoFs5MRNE4PoGMkMT5udM4JrnB5NJYFLJcDUUob363aj5e9.json".into(),
                "tests/data/sequential_blocks/mainnet-105495-3NL4zEKGtSokPMy29pGv7tm8uJt8GitM9JqrRg6Lkf3tRdnwrjpF.json".into(),
                "tests/data/sequential_blocks/mainnet-105496-3NK7yacg7pjHgV52sUmbNv9p7xxrKUV4sevy4Su5j6CrdTjyzaPL.json".into(),
                "tests/data/sequential_blocks/mainnet-105496-3NKE1aiFviFWrYMN5feKm3L7C4Zqp3czkwAtcXj1tdbaGDZ47L1k.json".into(),
                "tests/data/sequential_blocks/mainnet-105496-3NKK8yPXCULKBVyPebSJRjwiGTZTsoZhZX1DSfbSzV6VkCcZHygW.json".into(),
                "tests/data/sequential_blocks/mainnet-105496-3NKh1Y2S3wS3NYYLY6LsoP5hvQSHKL8wZ86GUM4r3NUWUdYt5h4Z.json".into(),
                "tests/data/sequential_blocks/mainnet-105496-3NKwSR9wWhXUwobCzCLEjHc5xQmvN1qkfnxG4EGoMGYRgYY5f9EB.json".into(),
                "tests/data/sequential_blocks/mainnet-105496-3NL4f5isMevxQHCRCSWSFey619Bkjwsf5R7CxmKEPLmWEJM9PXyS.json".into(),
                "tests/data/sequential_blocks/mainnet-105496-3NL5dFPzomwfNDA64vDzcyW5L49g2YPnXiBJ3XqZYFe87NDo9L1V.json".into(),
                "tests/data/sequential_blocks/mainnet-105497-3NKjngJTXJzRUXF3uH2nK19iYUVtYBFjLhezSrMMFVQyEGwqEi3c.json".into(),
                "tests/data/sequential_blocks/mainnet-105498-3NKbLiBHzQrAimK7AkP8qAfQpHnezkdsSm8mkt2TzsbjsLN8Axmt.json".into(),
                "tests/data/sequential_blocks/mainnet-105499-3NKEkf29fm6CARN6MAi6ZvmADxEXpu1wUwYfnjsiWCmR5LfCpwSg.json".into(),
                "tests/data/sequential_blocks/mainnet-105499-3NLmMoYPiS3oc6Vj3etc5xQd5Ny9cjcKCadqRqxeEHSRF5icw3es.json".into(),
                "tests/data/sequential_blocks/mainnet-105499-3NLmgdEg4HdPNzPNceezVrbahnW3yV2Wo6C8g49AddYUNnHBmd44.json".into(),
                "tests/data/sequential_blocks/mainnet-105499-3NLpfuGk5gvgaQuSQ3WrhXLX9mNJRZ1cNbRUAfCqdLqvVRjj4mL4.json".into(),
                "tests/data/sequential_blocks/mainnet-105500-3NK73T6brdpBFgjbZKMpfYX596q68sfHx8NtMDYRLJ9ai88WzrKQ.json".into(),
                "tests/data/sequential_blocks/mainnet-105500-3NKWrgDpSKN3DYSzRYGYmQofmT8Py99zYoVqhQHCZCGMjnBSuyup.json".into(),
                "tests/data/sequential_blocks/mainnet-105500-3NKqvNowLZT7Axe9Wn5o1uaJ93DErsriV8pHDzLHBLP6cRVmQfNP.json".into(),
                "tests/data/sequential_blocks/mainnet-105500-3NKvv2iBAPhZ8SRCxQEuGTgqTYuFXd2WVANXW6pcsR8pdzLuUj7C.json".into(),
                "tests/data/sequential_blocks/mainnet-105500-3NLYi7P4ZsGsvYsMp2vpyS97mGPBmBR8u7pH5wsffjks4143rguM.json".into(),
                "tests/data/sequential_blocks/mainnet-105500-3NLgP13DTcnuVpyEe65pjC6SS2upAzoKLW1cdas3w684b9FnpHxJ.json".into(),
                "tests/data/sequential_blocks/mainnet-105501-3NKBHgd9qR31HcnBRmyx5LDgXxhbmdVrfSbxtT8VJXBpQtdTsMev.json".into(),
                "tests/data/sequential_blocks/mainnet-105501-3NLJheWWdpapwu4HpYvwyhAFgyBzDWRPLLEZPi6veZineGyvDbwt.json".into(),
            ],
            orphaned_paths: vec![
                "tests/data/sequential_blocks/mainnet-105489-3NLFXtdzaFW2WX6KgrxMjL4enE4pCa9hAsVUPm47PT6337SXgBGh.json".into(),
                "tests/data/sequential_blocks/mainnet-105489-3NLUfaHDcyt9KsYxi1xsSdYE369GAduLxVgRUDE7RuFgSXQBphDK.json".into(),
            ],
        }
    );
}
