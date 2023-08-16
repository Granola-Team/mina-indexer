use mina_indexer::block::parser::FilesystemParser;
use std::path::PathBuf;
use tokio::time::Instant;

#[tokio::test]
async fn representative_benches() {
    let start = Instant::now();
    let sample_dir0 = PathBuf::from("./tests/data/non_sequential_blocks");
    let mut block_parser0 = FilesystemParser::new_testing(&sample_dir0).unwrap();
    let mut logs_processed = 0;

    while let Some(precomputed_block) = block_parser0
        .next()
        .await
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
    let mut block_parser1 = FilesystemParser::new_testing(&sample_dir1).unwrap();

    logs_processed = 0;
    while let Some(precomputed_block) = block_parser1
        .next()
        .await
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
    let mut block_parser = FilesystemParser::new_testing(&log_dir).unwrap();

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
