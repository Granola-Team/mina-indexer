use std::time::Duration;

use mina_indexer::receiver::{
    google_cloud::{GoogleCloudBlockReceiver, MinaNetwork},
    BlockReceiver,
};

#[tokio::test]
async fn receives_new_block() {
    let mut temp_block_dir = std::env::temp_dir();
    temp_block_dir.push("test_temp_block_dir");
    let bucket = "mina_network_block_data".to_string();
    tokio::fs::create_dir(&temp_block_dir).await.unwrap();

    let mut block_receiver = GoogleCloudBlockReceiver::new(
        1,
        1,
        &temp_block_dir,
        Duration::from_secs(1),
        MinaNetwork::Mainnet,
        bucket,
    )
    .await
    .expect("google cloud block receiver constructor succeeds");

    block_receiver
        .recv_block()
        .await
        .expect("block is received successfully");

    tokio::fs::remove_dir_all(&temp_block_dir)
        .await
        .expect("directory was created earlier in same test");
}
