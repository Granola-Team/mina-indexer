use std::path::PathBuf;

use mina_indexer::block::receiver::BlockReceiver;
use tokio::{
    fs::{create_dir, remove_dir, File},
    io::AsyncWriteExt,
};

const TEST_DIR: &'static str = "./receiver_tests";
const TEST_BLOCK: &'static str = include_str!(
    "../data/beautified_logs/mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json"
);

#[tokio::test]
async fn receiver_detects_new_block() {
    create_dir(TEST_DIR).await.unwrap();

    let test_dir_path = PathBuf::from(TEST_DIR);

    dbg!("initializing receiver");
    let mut block_receiver = BlockReceiver::new().await.unwrap();
    dbg!("loading directory");
    block_receiver.load_directory(&test_dir_path).await.unwrap();

    let mut test_block_path = test_dir_path;
    test_block_path.push("test_block.json");

    dbg!("creating file");
    let mut file = File::create(test_block_path).await.unwrap();
    file.write_all(TEST_BLOCK.as_bytes()).await.unwrap();

    dbg!("receiving block");
    let recvd = block_receiver.recv().await.unwrap().unwrap();

    dbg!(recvd);

    remove_dir(TEST_DIR).await.unwrap();
}
