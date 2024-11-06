use anyhow::Result;
use edgedb_derive::Queryable;
use std::{
    collections::{HashMap, VecDeque},
    fs::OpenOptions,
    io::Write,
    sync::Arc,
    time::Instant,
};
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use mina_indexer::db::DbPool;

const BLOCK_REPORTING_FREQ: usize = 1000;
const MAINNET_CANONICAL_THRESHOLD: usize = 290;
const WRITE_BUFFER_SIZE: usize = 10000;

#[derive(Debug, Queryable, Clone)]
#[allow(dead_code)]
struct Block {
    hash: String,
    previous_hash: String,
    blockchain_length: i64,
}

#[allow(dead_code)]
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let pool: Arc<DbPool> = Arc::new(DbPool::new(Some("trunk")).await?);
    let total = Instant::now();

    info!("Querying blocks from EdgeDB...");

    let blocks: Vec<Block> = pool
        .query(
            "SELECT Block {
                hash,
                previous_hash,
                blockchain_length
            }
            ORDER BY .blockchain_length DESC;",
            &(),
        )
        .await?;

    if blocks.is_empty() {
        error!("No blocks found in database");
        return Err(anyhow::anyhow!("No blocks found"));
    }

    info!("Retrieved {} blocks in {:?}", blocks.len(), total.elapsed());

    let block_map: HashMap<&str, &Block> = blocks.iter().map(|b| (b.hash.as_str(), b)).collect();

    info!("Finding highest canonical block...");
    let time = Instant::now();

    let mut highest_canonical_block = None;
    for block in blocks.iter() {
        // Start from the highest block
        let mut current_block = block;
        let mut chain_length = 0;

        while chain_length < MAINNET_CANONICAL_THRESHOLD {
            if let Some(parent) = block_map.get(current_block.previous_hash.as_str()) {
                chain_length += 1;
                current_block = parent;
            } else {
                break;
            }
        }

        if chain_length >= MAINNET_CANONICAL_THRESHOLD {
            highest_canonical_block = Some(block);
            break;
        }
    }

    let highest_canonical_block = match highest_canonical_block {
        Some(block) => block,
        None => {
            error!(
                "No block found with at least {} blocks in its chain",
                MAINNET_CANONICAL_THRESHOLD
            );
            return Err(anyhow::anyhow!("No suitable canonical block found"));
        }
    };

    info!(
        "Found highest canonical block at height {} in {:?}",
        highest_canonical_block.blockchain_length,
        time.elapsed()
    );

    // Now build the canonical chain starting from this block
    let mut canonical_chain: VecDeque<(&str, i64)> = VecDeque::new();
    let mut current_block = highest_canonical_block;

    while let Some(block) = block_map.get(current_block.hash.as_str()) {
        canonical_chain.push_front((block.hash.as_str(), block.blockchain_length));

        if canonical_chain.len() % BLOCK_REPORTING_FREQ == 0 {
            info!(
                "Found {} canonical blocks in {:?}",
                canonical_chain.len(),
                time.elapsed()
            );
        }

        if block.blockchain_length == 1 {
            break;
        }

        if let Some(parent) = block_map.get(block.previous_hash.as_str()) {
            current_block = parent;
        } else {
            warn!(
                "Gap found at block height {}. Ending chain discovery.",
                block.blockchain_length - 1
            );
            break;
        }
    }

    info!("Canonical chain discovery finished");
    info!(
        "Found {} blocks in the canonical chain in {:?}",
        canonical_chain.len(),
        time.elapsed()
    );

    // Write results to output file
    let mut output_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("chain.csv")?;

    let time = Instant::now();

    let mut buffer = String::new();
    for (hash, height) in canonical_chain.iter() {
        buffer.push_str(&format!("{},{}\n", hash, height));
        if buffer.len() >= WRITE_BUFFER_SIZE {
            output_file.write_all(buffer.as_bytes())?;
            buffer.clear();
        }
    }
    if !buffer.is_empty() {
        output_file.write_all(buffer.as_bytes())?;
    }

    output_file.flush()?;
    info!("{} written in {:?}", canonical_chain.len(), time.elapsed());
    info!("Total time: {:?}", total.elapsed());

    Ok(())
}
