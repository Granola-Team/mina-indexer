use std::{path::Path, time::Duration};

use mina_indexer::receiver::{filesystem::FilesystemReceiver, BlockReceiver};
use tokio::{
    fs::{create_dir_all, remove_dir_all, File},
    io::AsyncWriteExt,
    process::Command,
};

#[tokio::test]
async fn detects_new_block_written() {
    const TEST_BLOCK: &str = include_str!(
        "data/non_sequential_blocks/mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json"
    );

    let mut test_dir = std::env::temp_dir();
    test_dir.push("receiver_write_test");

    let mut success = false;
    let timeout = Duration::new(1, 0);

    let test_dir_path = test_dir.clone();
    let mut test_block_path = test_dir_path.clone();
    test_block_path.push("mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json");

    pretest(&test_dir).await;

    let mut block_receiver = FilesystemReceiver::new(1024, 64).await.unwrap();
    block_receiver.load_directory(&test_dir_path).unwrap();

    let mut file = File::create(test_block_path.clone()).await.unwrap();
    file.write_all(TEST_BLOCK.as_bytes()).await.unwrap();

    tokio::time::timeout(timeout, async {
        block_receiver.recv_block().await.unwrap().unwrap();
        success = true;
    })
    .await
    .unwrap();

    posttest(&test_dir, success).await;
}

#[tokio::test]
async fn detects_new_block_copied() {
    const TEST_BLOCK_PATH: &str = "./tests/data/non_sequential_blocks/mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json";

    let mut test_dir = std::env::temp_dir();
    test_dir.push("receiver_copy_test");

    let mut success = false;
    let timeout = Duration::new(1, 0);

    let mut test_block_path = test_dir.clone();
    test_block_path.push("mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json");

    pretest(&test_dir).await;

    let mut block_receiver = FilesystemReceiver::new(1024, 64).await.unwrap();
    block_receiver.load_directory(&test_dir).unwrap();

    tokio::time::sleep(Duration::from_secs(2)).await;

    let mut command = Command::new("cp")
        .arg(TEST_BLOCK_PATH)
        .arg(&test_block_path)
        .spawn()
        .unwrap();
    command.wait().await.unwrap();

    tokio::time::timeout(timeout, async {
        block_receiver.recv_block().await.unwrap().unwrap();
        success = true;
    })
    .await
    .unwrap();

    posttest(&test_dir, success).await;
}

async fn pretest(path: impl AsRef<Path>) {
    println!("Pretest: {}", path.as_ref().display());
    create_dir_all(path.as_ref()).await.unwrap_or(());
}

async fn posttest(path: impl AsRef<Path>, success: bool) {
    remove_dir_all(path.as_ref()).await.unwrap_or(());

    if !success {
        panic!("Timed out");
    }
}
