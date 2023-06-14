use std::path::PathBuf;

use mina_indexer::block::receiver::BlockReceiver;
use tokio::{
    fs::{create_dir, remove_dir, remove_file, File},
    io::AsyncWriteExt,
    process::Command,
};

#[tokio::test]
async fn detects_new_block_written() {
    const TEST_DIR: &'static str = "./receiver_write_test";
    const TEST_BLOCK: &'static str = include_str!(
        "../data/beautified_logs/mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json"
    );

    let test_dir_path = PathBuf::from(TEST_DIR);
    let mut test_block_path = test_dir_path.clone();
    test_block_path.push("mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json");

    if tokio::fs::metadata(&test_block_path).await.is_ok() {
        remove_file(test_block_path.clone()).await.unwrap();
        remove_dir(TEST_DIR).await.unwrap();
    }
    create_dir(TEST_DIR).await.unwrap_or(());

    let mut block_receiver = BlockReceiver::new().await.unwrap();

    block_receiver.load_directory(&test_dir_path).await.unwrap();

    let mut file = File::create(test_block_path.clone()).await.unwrap();
    file.write_all(TEST_BLOCK.as_bytes()).await.unwrap();

    let _recvd = block_receiver.recv().await.unwrap().unwrap();

    remove_file(test_block_path).await.unwrap();
    remove_dir(TEST_DIR).await.unwrap();
}

#[tokio::test]
async fn detects_new_block_copied() {
    const TEST_DIR: &'static str = "./receiver_copy_test";
    const TEST_BLOCK_PATH: &'static str = "./tests/data/beautified_logs/mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json";

    let test_dir_path = PathBuf::from(TEST_DIR);
    let mut test_block_path = test_dir_path.clone();
    test_block_path.push("mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json");

    if tokio::fs::metadata(&test_block_path).await.is_ok() {
        remove_file(test_block_path.clone()).await.unwrap();
        remove_dir(TEST_DIR).await.unwrap();
    }
    create_dir(TEST_DIR).await.unwrap_or(());

    let mut block_receiver = BlockReceiver::new().await.unwrap();

    block_receiver.load_directory(&test_dir_path).await.unwrap();

    let mut command = Command::new("cp")
        .arg(TEST_BLOCK_PATH)
        .arg(&test_block_path)
        .spawn()
        .unwrap();
    command.wait().await.unwrap();

    let _recvd = block_receiver.recv().await.unwrap().unwrap();

    remove_file(test_block_path).await.unwrap();
    remove_dir(TEST_DIR).await.unwrap();
}
