use std::{path::PathBuf, time::Duration};

use mina_indexer::block::receiver::BlockReceiver;
use tokio::{
    fs::{create_dir, metadata, remove_dir_all, File},
    io::AsyncWriteExt,
    process::Command,
};

#[tokio::test]
async fn detects_new_block_written() {
    const TEST_DIR: &'static str = "./receiver_write_test";
    const TEST_BLOCK: &'static str = include_str!(
        "../data/non_sequential_blocks/mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json"
    );

    let timeout = Duration::new(5, 0);
    let mut success = false;

    tokio::time::timeout(timeout, async {
        let test_dir_path = PathBuf::from(TEST_DIR);
        let mut test_block_path = test_dir_path.clone();
        test_block_path.push("mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json");

        pretest(TEST_DIR).await;

        let mut block_receiver = BlockReceiver::new().await.unwrap();
        block_receiver.load_directory(&test_dir_path).await.unwrap();

        let mut file = File::create(test_block_path.clone()).await.unwrap();
        file.write_all(TEST_BLOCK.as_bytes()).await.unwrap();

        block_receiver.recv().await.unwrap().unwrap();
        success = true;
    })
    .await
    .unwrap();

    posttest(TEST_DIR, success).await;
}

#[tokio::test]
async fn detects_new_block_copied() {
    const TEST_DIR: &'static str = "./receiver_copy_test";
    const TEST_BLOCK_PATH: &'static str = "./tests/data/non_sequential_blocks/mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json";

    let timeout = Duration::new(5, 0);
    let mut success = false;

    tokio::time::timeout(timeout, async {
        let test_dir_path = PathBuf::from(TEST_DIR);
        let mut test_block_path = test_dir_path.clone();
        test_block_path.push("mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json");

        pretest(TEST_DIR).await;

        let mut block_receiver = BlockReceiver::new().await.unwrap();
        block_receiver.load_directory(&test_dir_path).await.unwrap();

        let mut command = Command::new("cp")
            .arg(TEST_BLOCK_PATH)
            .arg(&test_block_path)
            .spawn()
            .unwrap();
        command.wait().await.unwrap();

        block_receiver.recv().await.unwrap().unwrap();
        success = true;
    })
    .await
    .unwrap();

    posttest(TEST_DIR, success).await;
}

async fn pretest(path: &str) {
    if metadata(path).await.is_ok() {
        remove_dir_all(path).await.unwrap();
    }
    create_dir(path).await.unwrap_or(());
}

async fn posttest(path: &str, success: bool) {
    if metadata(path).await.is_ok() {
        remove_dir_all(path).await.unwrap();
    }

    if !success {
        panic!("Timed out");
    }
}
