use bytesize::ByteSize;
use clap::Parser;
use mina_indexer::{
    block::{parser::BlockParser, BlockHash},
    display_duration,
    state::{ledger::genesis, IndexerMode, IndexerState},
    store::IndexerStore,
    CANONICAL_UPDATE_THRESHOLD, MAINNET_CANONICAL_THRESHOLD, MAINNET_TRANSITION_FRONTIER_K,
    PRUNE_INTERVAL_DEFAULT,
};
use std::{fs, path::PathBuf, sync::Arc, thread};
use tokio::{
    process,
    time::{Duration, Instant},
};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::prelude::*;

// Assumed to be run on a block directory with almost all mainnet blocks.

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Startup blocks directory path
    #[arg(short, long, default_value = concat!(env!("HOME"), ".mina-indexer/startup-blocks"))]
    blocks_dir: PathBuf,
    /// Watch blocks directory path
    #[arg(short, long, default_value = concat!(env!("HOME"), ".mina-indexer/watch-blocks"))]
    watch_dir: PathBuf,
    /// Max blockchain_length of blocks to parse
    #[arg(short = 'l', long, default_value_t = 10_000)]
    max_block_length: u32,
    /// Max number of blocks to parse
    #[arg(short = 'n', long, default_value_t = 10_000)]
    max_block_count: u32,
    /// Max console log level
    #[arg(long, default_value_t = LevelFilter::INFO)]
    log_level: LevelFilter,
    /// Report frequency (number of blocks)
    #[arg(short, long, default_value_t = 5000)]
    report_freq: u32,
    /// Watch duration (sec)
    #[arg(short, long, default_value_t = 0)]
    duration: u64,
    /// To keep the db or not, that is the question
    #[arg(short, long, default_value_t = false)]
    persist_db: bool,
    /// Indexer mode
    #[arg(long, default_value_t = IndexerMode::Light)]
    mode: IndexerMode,
    /// Verbose output
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

const DB_PATH: &str = "./mainnet-test-block-store";
const GENESIS_HASH: &str = "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let blocks_dir = args.blocks_dir;
    let watch_dir = args.watch_dir;
    let max_block_length = args.max_block_length;
    let max_block_count = args.max_block_count;
    let log_level = args.log_level;
    let freq = args.report_freq;
    let duration = args.duration;
    let persist_db = args.persist_db;
    let mode = args.mode;
    let verbose = args.verbose;

    assert!(blocks_dir.is_dir(), "Should be a dir path");

    let stdout_layer = tracing_subscriber::fmt::layer();
    tracing_subscriber::registry()
        .with(stdout_layer.with_filter(log_level))
        .init();

    let mut bp =
        BlockParser::new_filtered(&blocks_dir, max_block_length, MAINNET_CANONICAL_THRESHOLD)
            .unwrap();
    let store_dir = &PathBuf::from(DB_PATH);
    if store_dir.exists() {
        fs::remove_dir_all(store_dir)?;
    }

    let indexer_store = Arc::new(IndexerStore::new(store_dir).unwrap());
    let genesis_path = &PathBuf::from("./tests/data/genesis_ledgers/mainnet.json");
    let parse_genesis_time = Instant::now();
    let genesis_root = genesis::parse_file(genesis_path).await.unwrap();
    let parse_genesis_time = parse_genesis_time.elapsed();

    println!("Genesis ledger parsing time: {parse_genesis_time:?}");

    let total_time = Instant::now();
    let mut state = IndexerState::new(
        mode,
        BlockHash(GENESIS_HASH.to_string()),
        genesis_root.ledger,
        indexer_store,
        MAINNET_TRANSITION_FRONTIER_K,
        PRUNE_INTERVAL_DEFAULT,
        CANONICAL_UPDATE_THRESHOLD,
    )
    .unwrap();

    let sorting_time = total_time.elapsed();
    println!("Sorting time: {sorting_time:?}\n");

    // blocks
    let mut block_count = 1;
    let mut highest_seq_height = 2;

    // branches
    let mut max_branches = 1;
    let mut max_root_len = 0;
    let mut max_root_height = 0;
    let mut max_dangling_len = 0;
    let mut max_dangling_height = 0;

    // time
    let mut floor_minutes = 0;
    let mut adding_time = Duration::new(0, 0);
    let mut parsing_time = Duration::new(0, 0);

    for _ in 2..max_block_count.min(max_block_length) {
        let display_elapsed = display_duration(total_time.elapsed());

        // Report every passing minute
        if verbose && total_time.elapsed().as_secs() % 60 > floor_minutes {
            println!("Time elapsed: {display_elapsed}");
            floor_minutes += 1;
        }

        // Report every freq blocks
        if block_count % freq == 0 {
            println!("=== Progress #{} ===", block_count / freq);
            println!("Blocks:  {block_count}");
            println!("Total:   {display_elapsed}");

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
                state.add_block(&block).unwrap();
                adding_time += add.elapsed();

                if verbose {
                    println!(
                        "Added block (length: {}, state_hash: {:?})",
                        block.blockchain_length, block.state_hash
                    );
                    match block.blockchain_length.cmp(&highest_seq_height) {
                        std::cmp::Ordering::Less => {
                            println!(
                                "Another block of height: {}! state_hash: {:?}",
                                block.blockchain_length, block.state_hash
                            )
                        }
                        std::cmp::Ordering::Equal => highest_seq_height += 1,
                        std::cmp::Ordering::Greater => {
                            println!("Expected {highest_seq_height}, instead got height {}, state_hash: {:?}", block.blockchain_length, block.state_hash)
                        }
                    }
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

    println!("~~~~~~~~~~~~~~~~~~");
    println!("~~~ Benchmarks ~~~");
    println!("~~~~~~~~~~~~~~~~~~");
    println!("Sorting: {sorting_time:?}");
    println!("Blocks:  {block_count}");
    println!("Total:   {total_time:?}");

    println!("\n~~~ Parsing ~~~");
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

    println!("Estimated time to ingest all mainnet blocks at this rate:");
    println!(
        "{} hrs",
        (640_000. * total_add.as_secs_f64()) / (3600. * block_count as f64)
    );

    println!("\n~~~ DB stats ~~~");
    println!(
        "Estimate number of keys:    {:?}",
        state
            .indexer_store
            .as_ref()
            .unwrap()
            .estimate_live_data_size()
    );
    println!(
        "Estimate live data size:    {:?}",
        ByteSize::b(
            state
                .indexer_store
                .as_ref()
                .unwrap()
                .estimate_live_data_size()
        )
    );
    println!(
        "Current size all memtables: {:?}",
        ByteSize::b(
            state
                .indexer_store
                .as_ref()
                .unwrap()
                .cur_size_all_mem_tables()
        )
    );
    println!("{}", state.indexer_store.as_ref().unwrap().db_stats());

    println!("Initial ingestion complete!");
    println!("Watching {} now", watch_dir.display());

    let mut next_length = state.best_tip_block().blockchain_length + 1;
    let watch_duration = Duration::new(duration, 0);
    let watch_time = Instant::now();

    // get next blocks in a loop via gsutil
    loop {
        if watch_time.elapsed() >= watch_duration {
            break;
        }

        let mut command = process::Command::new("gsutil");
        command.arg("-m");
        command.arg("cp");
        command.arg("-n");
        command.arg(&format!(
            "gs://mina_network_block_data/mainnet-{next_length}-*.json"
        ));
        command.arg(&watch_dir.display().to_string());

        let mut cmd = command.spawn()?;
        if let Ok(exit_status) = cmd.wait().await {
            if exit_status.success() {
                next_length += 1;
            }
        }
        thread::sleep(Duration::new(180, 0));
    }

    if !persist_db {
        tokio::fs::remove_dir_all(store_dir).await.unwrap();
    }

    Ok(())
}
