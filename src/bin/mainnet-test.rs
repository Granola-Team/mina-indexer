use std::path::PathBuf;

use mina_indexer::block::{parser::BlockParser, BlockHash};
use mina_indexer::state::IndexerState;

use mina_indexer::state::ledger::genesis;
use tokio::time::{Duration, Instant};

use bytesize::ByteSize;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    let log_dir = args[1]
        .parse::<PathBuf>()
        .expect("First arg should be block log dir path");
    let max_block_count = args[2]
        .parse::<u32>()
        .expect("Second arg should be number of blocks");

    let mut bp = BlockParser::new(&log_dir).unwrap();

    const DB_PATH: &str = "./block-store-test";
    let store_dir = &PathBuf::from(DB_PATH);

    const GENESIS_HASH: &str = "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ";
    let genesis_path = &PathBuf::from("./tests/data/genesis_ledgers/mainnet.json");
    let genesis_root = genesis::parse_file(genesis_path).await.unwrap();

    let mut state = IndexerState::new(
        BlockHash(GENESIS_HASH.to_string()),
        Some(genesis_root.ledger),
        Some(&PathBuf::from(store_dir)),
    )
    .unwrap();

    let mut max_branches = 1;
    let mut max_dangling_len = 0;
    let mut max_dangling_height = 0;
    let mut block_count = 1;
    let total = Instant::now();
    let mut adding = Duration::new(0, 0);

    for _ in 1..max_block_count {
        match bp.next().await {
            Err(err) => {
                println!("{err:?}");
            }
            Ok(None) => {
                println!("Ran out of blocks");
                break;
            }
            Ok(Some(block)) => {
                let add = Instant::now();
                let _ext = state.add_block(&block).unwrap();
                adding += add.elapsed();
                block_count += 1;
                if state.dangling_branches.len() + 1 > max_branches {
                    max_branches = state.dangling_branches.len() + 1;
                }
                for dangling in &state.dangling_branches {
                    if dangling.len() > max_dangling_len {
                        max_dangling_len = dangling.len();
                    }
                    if dangling.height() > max_dangling_height {
                        max_dangling_height = dangling.height();
                    }
                }
            }
        }
    }

    let total_add = adding;
    let total_time = total.elapsed();

    println!("\n~~~~~~~~~~~~~~~~~~");
    println!("~~~ Benchmarks ~~~");
    println!("~~~~~~~~~~~~~~~~~~");
    println!("Blocks:  {block_count}");
    println!("Total:   {total_time:?}");

    let blocks_per_sec = block_count as f64 / total_add.as_secs_f64();
    println!("\n~~~ Add to state ~~~");
    println!("Avg:     {:?}", total_add / block_count);
    println!("Total:   {total_add:?}");
    println!("Per sec: {blocks_per_sec:?} blocks");
    println!("Per hr:  {:?} blocks", blocks_per_sec * 3600.);

    println!("\n~~~ Branches ~~~");
    println!("Max num:             {max_branches}");
    println!(
        "Root height:         {}",
        state.root_branch.as_ref().unwrap().height()
    );
    println!(
        "Root length:         {}",
        state.root_branch.as_ref().unwrap().len()
    );
    println!("Max dangling len:    {max_dangling_len}");
    println!("Max dangling height: {max_dangling_height}\n");

    println!("Estimated time to ingest all (~260_000) mainnet blocks at this rate:");
    println!(
        "{} hrs\n",
        (260_000. * total_add.as_secs_f64()) / (3600. * block_count as f64)
    );

    println!("\n~~~ DB stats ~~~");
    let db_stats = state
        .block_store
        .as_ref()
        .unwrap()
        .database
        .property_value(rocksdb::properties::DBSTATS)
        .unwrap()
        .unwrap();
    let est_data_size = state
        .block_store
        .as_ref()
        .unwrap()
        .database
        .property_int_value(rocksdb::properties::ESTIMATE_LIVE_DATA_SIZE)
        .unwrap()
        .unwrap();
    let est_num_keys = state
        .block_store
        .as_ref()
        .unwrap()
        .database
        .property_int_value(rocksdb::properties::ESTIMATE_NUM_KEYS)
        .unwrap()
        .unwrap();
    let curr_size_mem = state
        .block_store
        .as_ref()
        .unwrap()
        .database
        .property_int_value(rocksdb::properties::CUR_SIZE_ALL_MEM_TABLES)
        .unwrap()
        .unwrap();

    println!("Estimate live data size:    {est_data_size:?}");
    println!("Estimate number of keys:    {est_num_keys:?}");
    println!(
        "Current size all memtables: {:?}",
        ByteSize::b(curr_size_mem)
    );
    println!("{db_stats}");

    tokio::fs::remove_dir_all(store_dir).await.unwrap();
}
