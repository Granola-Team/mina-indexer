use std::{fs, sync::Arc, thread};

use crossbeam_channel::bounded;
use mina_indexer::{block::ingestion, mina_blocks::v1::precomputed_block::parse_file};
use tempfile::TempDir;

#[test]
fn block_ingestion_watch_blocks() -> anyhow::Result<()> {
    let block_src = "tests/data/non_sequential_blocks/mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json";
    let (watch_tx, watch_rx) = bounded(16);

    // create tmp dir for test
    let tmp_dir = Arc::new(TempDir::new()?);
    let tmp_dir_copy = tmp_dir.clone();
    fs::create_dir_all(tmp_dir.path().clone()).expect("Temp directory should have been created");
    let block_dst = tmp_dir
        .path()
        .join("mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json");
    // Spawn block watcher in a different thread
    thread::spawn(move || {
        let _ = ingestion::watch_directory_for_blocks(&tmp_dir.path().clone(), watch_tx);
    });
    // Copy file to the watch dir for consumption
    fs::copy(block_src, block_dst)?;

    // File has been processed and sent downstream to there
    let seen_block = watch_rx.recv()?;

    let pcb_expected = parse_file(block_src)?;
    let pcb_actual = parse_file(seen_block)?;

    fs::remove_dir_all(tmp_dir_copy.path()).expect("Temp directory should have been removed");
    assert_eq!(pcb_expected, pcb_actual);
    Ok(())
}
