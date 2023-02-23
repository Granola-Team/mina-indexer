use std::path::PathBuf;

use mina_indexer::precomputed_block::{
    scanner::LogScanner, BlockLog, LogEntryProcessor, PrecomputedBlock,
};
use tokio::io::AsyncReadExt;

#[tokio::test]
async fn block_logs_scan_correctly() {
    let logs_dir = PathBuf::from("./tests/data/block_logs");
    let logs_scanner = LogScanner::new(&logs_dir);
    let mut num_entries = 0;
    for entry in logs_scanner.log_files() {
        num_entries += 1;
        dbg!(entry);
    }

    assert_eq!(num_entries, 23);
}

#[tokio::test]
async fn block_log_deserialization() {
    let block_log = PathBuf::from("./tests/data/block_logs/mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json");
    let mut log_file = tokio::fs::File::open(&block_log)
        .await
        .expect("file exists");

    let mut contents = Vec::new();
    log_file
        .read_to_end(&mut contents)
        .await
        .expect("able to read log file!");

    let block: BlockLog = serde_json::from_slice(&contents).expect("deserialization failed :(");
    dbg!(block);
}

#[tokio::test]
async fn block_log_integration() {
    let logs_dir = PathBuf::from("./tests/data/block_logs");
    let logs_scanner = LogScanner::new(&logs_dir);
    let logs_processor = LogEntryProcessor::new(Box::new(logs_scanner.log_files()));
    for precomputed_block in logs_processor.parse_log_entries().await {
        dbg!(precomputed_block.state_hash);
    }
}
