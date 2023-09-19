use std::{path::Path, time::Duration};

use mina_indexer::receiver::{filesystem::FilesystemReceiver, BlockReceiver};
use tokio::{
    fs::{create_dir, metadata, remove_dir_all, File},
    io::AsyncWriteExt,
    process::Command,
};

#[tokio::test]
async fn detects_new_block_written() {
    let mut test_dir = std::env::temp_dir();
    test_dir.push("receiver_write_test");
    const TEST_BLOCK: &'static str = include_str!(
        "data/non_sequential_blocks/mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json"
    );

    let timeout = Duration::new(5, 0);
    let mut success = false;

    let test_dir_path = test_dir.clone();
    let mut test_block_path = test_dir_path.clone();
    test_block_path.push("mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json");

    pretest(&test_dir).await;

    let mut block_receiver = FilesystemReceiver::new(1024, 64).await.unwrap();
    block_receiver.load_directory(&test_dir_path).unwrap();

    tokio::time::sleep(Duration::from_secs(2)).await;

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
    let mut test_dir = std::env::temp_dir();
    test_dir.push("receiver_copy_test");
    const TEST_BLOCK_PATH: &'static str = "./tests/data/non_sequential_blocks/mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json";

    let timeout = Duration::new(5, 0);
    let mut success = false;

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
    if metadata(path.as_ref()).await.is_ok() {
        remove_dir_all(path.as_ref()).await.unwrap();
    }
    create_dir(path.as_ref()).await.unwrap_or(());
}

async fn posttest(path: impl AsRef<Path>, success: bool) {
    if metadata(path.as_ref()).await.is_ok() {
        remove_dir_all(path.as_ref()).await.unwrap();
    }

    if !success {
        panic!("Timed out");
    }
}
