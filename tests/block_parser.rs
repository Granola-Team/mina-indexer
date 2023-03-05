use std::path::PathBuf;

use mina_indexer::block_parser::BlockParser;

#[tokio::test]
async fn block_parser_parses_representative_sample() {
    let sample_dir = PathBuf::from("./tests/data/beautified_logs");
    let mut block_parser = BlockParser::new(&sample_dir);
    let mut logs_processed = 0;
    while let Some(precomputed_block) = block_parser.next().await.expect("IO Error on block_parser")
    {
        logs_processed += 1;
        dbg!(precomputed_block.state_hash);
    }

    assert_eq!(logs_processed, 23);
}
