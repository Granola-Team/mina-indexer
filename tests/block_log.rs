use mina_indexer::block_log::reader::{filesystem_json::FilesystemJSONReader, BlockLogReader};

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
