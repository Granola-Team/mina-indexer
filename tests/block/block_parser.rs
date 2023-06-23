use std::path::PathBuf;

use mina_indexer::block::parser::BlockParser;
use tokio::time::Instant;

#[tokio::test]
async fn representative_bench() {
    let start = Instant::now();
    let sample_dir0 = PathBuf::from("./tests/data/block_logs");
    let mut block_parser0 = BlockParser::new_testing(&sample_dir0).unwrap();
    let mut logs_processed = 0;
    while let Some(precomputed_block) = block_parser0
        .next()
        .await
        .expect("IO Error on block_parser")
    {
        logs_processed += 1;
        dbg!(precomputed_block.state_hash);
    }
    assert_eq!(logs_processed, block_parser0.total_num_blocks);
    println!("./tests/data/beautified_logs");
    println!("Parse {logs_processed} logs: {:?}\n", start.elapsed());

    let start = Instant::now();
    logs_processed = 0;
    let sample_dir1 = PathBuf::from("./tests/data/sequential_blocks");
    let mut block_parser1 = BlockParser::new_testing(&sample_dir1).unwrap();
    while let Some(precomputed_block) = block_parser1
        .next()
        .await
        .expect("IO Error on block_parser")
    {
        logs_processed += 1;
        dbg!(precomputed_block.state_hash);
    }
    assert_eq!(logs_processed, block_parser1.total_num_blocks);
    println!("./tests/data/sequential_blocks");
    println!("Parse {logs_processed} logs: {:?}\n", start.elapsed());
}
