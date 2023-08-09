use std::{path::Path, time::Duration};

use mina_indexer::receiver::{google_cloud::{GoogleCloudBlockReceiver, MinaNetwork}, BlockReceiver};

#[tokio::test]
async fn receives_new_block() {
    let temp_block_dir: &Path = &Path::new("./test_temp_block_dir");
    let bucket = "mina_network_block_data".to_string();
    tokio::fs::create_dir(temp_block_dir).await.expect("parent directory exists");

    let mut block_receiver = GoogleCloudBlockReceiver::new(
       1, 1, temp_block_dir.into(), Duration::from_secs(1), MinaNetwork::Mainnet, bucket
    ).await.expect("google cloud block receiver constructor succeeds");

    block_receiver.recv_block().await.expect("block is received successfully");
    drop(block_receiver);
}