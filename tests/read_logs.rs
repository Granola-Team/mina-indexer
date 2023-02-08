#[tokio::test]
async fn test_block_deserialization() {
    let block_logs_prefix = "./tests/data/block_logs";
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

        mina_indexer::blocks::read_block_log(&entry_path)
            .await
            .expect("block log deserializes from JSON");
    }
}
