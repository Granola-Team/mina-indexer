use std::path::PathBuf;

use mina_indexer::block_log::{
    reader::{filesystem_json::FilesystemJSONReader, BlockLogReader},
    scanner::LogScanner,
};

#[tokio::test]
async fn block_logs_parse_correctly() {
    let logs_dir = "./tests/data/block_logs";
    let mut logs_reader = FilesystemJSONReader::new(logs_dir)
        .await
        .expect("io error on FilesystemJSONReader::new()");

    while let Some(block_log) = logs_reader
        .next_log()
        .await
        .expect("io error on next_log()")
    {
        dbg!(block_log.state_hash);
    }
}

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
