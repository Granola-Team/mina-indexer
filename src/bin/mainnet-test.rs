use bytesize::ByteSize;
use clap::Parser;
use mina_indexer::{
    block::{parser::BlockParser, BlockHash},
    state::{ledger::genesis, IndexerState},
    MAINNET_TRANSITION_FRONTIER_K,
};
use std::path::PathBuf;
use tokio::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    blocks_dir: PathBuf,
    #[arg(short, long, default_value_t = 10_000)]
    max_block_count: u32,
    #[arg(short, long, default_value_t = 5000)]
    report_freq: u32,
    #[arg(short, long, default_value_t = false)]
    persist_db: bool,
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let blocks_dir = args.blocks_dir;
    let freq = args.report_freq;

    assert!(blocks_dir.is_dir(), "Should be a dir path");

    let max_block_count = args.max_block_count;

    let mut bp = BlockParser::new(&blocks_dir).unwrap();

    const DB_PATH: &str = "./mainnet-test-block-store";
    let store_dir = &PathBuf::from(DB_PATH);

    const GENESIS_HASH: &str = "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ";
    let genesis_path = &PathBuf::from("./tests/data/genesis_ledgers/mainnet.json");

    let parse_genesis_time = Instant::now();
    let genesis_root = genesis::parse_file(genesis_path).await.unwrap();
    let parse_genesis_time = parse_genesis_time.elapsed();
    println!("Genesis ledger parsing time: {parse_genesis_time:?}");

    let total_time = Instant::now();
    let mut state = IndexerState::new(
        BlockHash(GENESIS_HASH.to_string()),
        genesis_root.ledger,
        Some(&PathBuf::from(store_dir)),
        Some(MAINNET_TRANSITION_FRONTIER_K),
        None,
    )
    .unwrap();
    let sorting_time = total_time.elapsed();
    println!("Sorting time: {sorting_time:?}\n");

    let mut max_branches = 1;
    let mut max_root_len = 0;
    let mut max_root_height = 0;
    let mut max_dangling_len = 0;
    let mut max_dangling_height = 0;
    let mut block_count = 1;
    let mut highest_seq_height = 2;

    let mut floor_minutes = 0;
    let mut adding_time = Duration::new(0, 0);
    let mut parsing_time = Duration::new(0, 0);

    for _ in 1..max_block_count {
        // Report every passing minute
        if args.verbose && total_time.elapsed().as_secs() % 60 > floor_minutes {
            println!("Time elapsed: {:?}", total_time.elapsed());
            floor_minutes += 1;
        }

        // Report every freq blocks
        if block_count % freq == 0 {
            println!("=== Progress #{} ===", block_count / freq);
            println!("Blocks:  {block_count}");
            println!("Total:   {:?}", total_time.elapsed());

            let blocks_per_sec = block_count as f64 / adding_time.as_secs_f64();
            println!("\n~~~ Add to state ~~~");
            println!("Avg:     {:?}", adding_time / block_count);
            println!("Total:   {adding_time:?}");
            println!("Per sec: {blocks_per_sec:?} blocks");
            println!("Per hr:  {:?} blocks", blocks_per_sec * 3600.);

            println!("\n~~~ Branches ~~~");
            println!("Max num:             {max_branches}");
            println!("Max root length:     {max_root_len}");
            println!("Max root height:     {max_root_height}");
            println!("Max dangling length: {max_dangling_len}");
            println!("Max dangling height: {max_dangling_height}\n");
        }

        let parse_time = Instant::now();
        match bp.next().await {
            Err(err) => {
                println!("{err:?}");
            }
            Ok(None) => {
                println!("Ran out of blocks");
                break;
            }
            Ok(Some(block)) => {
                parsing_time += parse_time.elapsed();

                let add = Instant::now();
                state.add_block(&block, true).unwrap();
                adding_time += add.elapsed();

                match block.blockchain_length {
                    None => println!("Block with no height! state_hash: {:?}", block.state_hash),
                    Some(n) => match n.cmp(&highest_seq_height) {
                        std::cmp::Ordering::Less => {
                            println!(
                                "Another block of height: {n}! state_hash: {:?}",
                                block.state_hash
                            );
                        }
                        std::cmp::Ordering::Equal => highest_seq_height += 1,
                        std::cmp::Ordering::Greater => {
                            println!("Expected {highest_seq_height}, instead got height {n}, state_hash: {:?}", block.state_hash);
                        }
                    },
                }

                if args.verbose {
                    println!(
                        "Added block (length: {}, state_hash: {:?})",
                        block.blockchain_length.unwrap_or(0),
                        block.state_hash
                    );
                }

                // update metrics
                block_count += 1;
                max_branches = state.dangling_branches.len().max(max_branches);
                max_root_height = state.root_branch.height().max(max_root_height);
                max_root_len = state.root_branch.len().max(max_root_len);
                for dangling in &state.dangling_branches {
                    max_dangling_height = dangling.height().max(max_dangling_height);
                    max_dangling_len = dangling.len().max(max_dangling_len);
                }
            }
        }
    }

    let total_add = adding_time;
    let total_time = total_time.elapsed();

    println!("\n~~~~~~~~~~~~~~~~~~");
    println!("~~~ Benchmarks ~~~");
    println!("~~~~~~~~~~~~~~~~~~");
    println!("Sorting: {sorting_time:?}");
    println!("Blocks:  {block_count}");
    println!("Total:   {total_time:?}");

    println!("~~~ Parsing ~~~");
    println!("Genesis ledger: {parse_genesis_time:?}");
    println!("Blocks:         {parsing_time:?}");

    let blocks_per_sec = block_count as f64 / total_add.as_secs_f64();
    println!("\n~~~ Add to state ~~~");
    println!("Avg:     {:?}", total_add / block_count);
    println!("Total:   {total_add:?}");
    println!("Per sec: {blocks_per_sec:?} blocks");
    println!("Per hr:  {:?} blocks", blocks_per_sec * 3600.);

    println!("\n~~~ Branches ~~~");
    println!("Max num:             {max_branches}");
    println!("Root height:         {}", &state.root_branch.height());
    println!("Root length:         {}", &state.root_branch.len());
    println!("Max dangling length: {max_dangling_len}");
    println!("Max dangling height: {max_dangling_height}\n");

    println!("Estimated time to ingest all (~640_000) mainnet blocks at this rate:");
    println!(
        "{} hrs",
        (640_000. * total_add.as_secs_f64()) / (3600. * block_count as f64)
    );

    println!("\n~~~ DB stats ~~~");
    println!(
        "Estimate number of keys:    {:?}",
        state
            .block_store
            .as_ref()
            .unwrap()
            .estimate_live_data_size()
    );
    println!(
        "Estimate live data size:    {:?}",
        ByteSize::b(
            state
                .block_store
                .as_ref()
                .unwrap()
                .estimate_live_data_size()
        )
    );
    println!(
        "Current size all memtables: {:?}",
        ByteSize::b(
            state
                .block_store
                .as_ref()
                .unwrap()
                .cur_size_all_mem_tables()
        )
    );
    println!("{}", state.block_store.as_ref().unwrap().db_stats());

    if !args.persist_db {
        tokio::fs::remove_dir_all(store_dir).await.unwrap();
    }
}
