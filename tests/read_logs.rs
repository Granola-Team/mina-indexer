use std::path::Path;

use mina_indexer::blocks::LogsProcessor;

#[tokio::test]
async fn test_block_deserialization() {
    let block_logs_prefix = "./tests/data/logs_raw";
    let mut blocks_dir = tokio::fs::read_dir(block_logs_prefix)
        .await
        .expect("testing logs dir exists");
    while let Some(entry) = blocks_dir
        .next_entry()
        .await
        .expect("reading logs dir shouldn't cause IO error")
    {
        let entry_type = entry.file_type().await.expect("entry has a file type");
        assert!(entry_type.is_file());
        let entry_name = entry
            .file_name()
            .to_str()
            .expect("file name exists")
            .to_string();
        let entry_path = format!("{}/{}", block_logs_prefix, entry_name);
        println!("attempting to parse block log at {:?}", entry_path);

        let value = mina_indexer::blocks::read_block_log(&Path::new(&entry_path))
            .await
            .expect("block log deserializes from JSON");

        dbg!(&value["protocol_state"]["previous_state_hash"]);
    }
}

#[tokio::test]
async fn test_logs_processor() {
    let block_logs_prefix = "./tests/data/logs_raw";
    let mut logs_processor = LogsProcessor::new(block_logs_prefix)
        .await
        .expect("logs processor initializes successfully");

    while let Some(block_log) = logs_processor.next_log().await.expect("no errors produced") {
        dbg!(block_log.log_name, logs_processor.logs_processed);
    }
}
