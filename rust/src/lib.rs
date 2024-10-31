use bigdecimal::BigDecimal;
use db::{DbPool, MAX_CONNECTIONS};
use futures::future::try_join_all;
use rayon::prelude::*;
use sonic_rs::{JsonType, JsonValueTrait, Value};
use stats::ProcessingStats;
use std::{
    collections::HashSet, fs, future::Future, io, os::unix::fs::MetadataExt, path::PathBuf,
    str::FromStr, sync::Arc, time::Instant,
};
use walkdir::WalkDir;

pub mod blocks;
mod db;
pub mod staking;
pub mod stats;

const ACCOUNTS_BATCH_SIZE: usize = MAX_CONNECTIONS / 3;

const BLOCK_FILE_PREFIX: &str = "mainnet-";

#[inline]
pub(crate) fn chunk_size() -> usize {
    let cpu_count = num_cpus::get();
    std::cmp::min(32, std::cmp::max(8, cpu_count * 2))
}

#[inline]
fn get_file_paths(dir: &str) -> Result<Vec<PathBuf>, io::Error> {
    let mut paths: Vec<PathBuf> = Vec::with_capacity(900_000);

    WalkDir::new(dir)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path().extension().map_or(false, |ext| ext == "json")
                && e.file_name()
                    .to_str()
                    .map_or(false, |name| name.starts_with(BLOCK_FILE_PREFIX))
        })
        .for_each(|e| paths.push(e.into_path()));

    // Sort by block number (second part of the filename)
    paths.par_sort_unstable_by(|a, b| {
        let get_block_num = |p: &PathBuf| -> u32 {
            p.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .split('-')
                .nth(1) // Get the second part after splitting by '-'
                .and_then(|s| s.parse().ok())
                .unwrap_or(0)
        };

        let a_num = get_block_num(a);
        let b_num = get_block_num(b);

        a_num.cmp(&b_num)
    });

    Ok(paths)
}

pub async fn process_files<F, Fut>(dir: &str, pool: Arc<DbPool>, processor: F) -> anyhow::Result<()>
where
    F: Fn(Arc<DbPool>, Value, String, i64) -> Fut + Send + Sync + Clone + 'static,
    Fut: Future<Output = Result<(), edgedb_tokio::Error>> + Send,
{
    let paths = get_file_paths(dir)?;
    let chunks: Vec<_> = paths.chunks(chunk_size()).map(|c| c.to_vec()).collect();
    let mut stats = ProcessingStats::new(chunks.len());

    for (chunk_index, chunk) in chunks.iter().enumerate() {
        let chunk_start = Instant::now();
        let mut chunk_handles = vec![];

        for path in chunk {
            let pool = Arc::clone(&pool);
            let path = path.clone();
            let processor = processor.clone();

            let handle = tokio::spawn(async move {
                match to_json(&path).await {
                    Ok(json) => {
                        let hash = extract_hash_from_file_name(&path);
                        let number = extract_digits_from_file_name(&path);

                        match processor(pool, json, hash.clone(), number).await {
                            Ok(_) => println!("Processed: {}", hash),
                            Err(e) => eprintln!("Error processing {}: {:?}", hash, e),
                        }
                    }
                    Err(e) => match e.kind() {
                        std::io::ErrorKind::InvalidData => {
                            println!("Error - Contains invalid UTF-8 data: {:?}", &path);
                        }
                        _ => println!("Error - Failed to read file {:?}: {}", &path, e),
                    },
                }
            });

            chunk_handles.push(handle);
        }

        for handle in chunk_handles {
            if let Err(e) = handle.await {
                eprintln!("Task failed: {:?}", e);
            }
        }

        stats.update(chunk_start.elapsed());
        let pool_stats = pool.get_pool_stats().await;

        println!(
            "Chunk {}/{} completed. Pool stats: {:?}. {}",
            chunk_index + 1,
            chunks.len(),
            pool_stats,
            stats.get_stats()
        );
    }

    Ok(())
}

fn to_titlecase(s: &str) -> String {
    s.chars()
        .next()
        .map(|c| c.to_ascii_uppercase())
        .into_iter()
        .chain(s.chars().skip(1))
        .collect()
}

/// Extract the hash part from a Mina block or staking ledger file name
fn extract_hash_from_file_name(path: &PathBuf) -> String {
    let file_name = path.file_name().unwrap().to_str().unwrap();
    file_name
        .split('-')
        .nth(2)
        .unwrap()
        .split('.')
        .next()
        .unwrap()
        .to_string()
}

/// Extract the digits part from a Mina block (the height) or staking ledger (the epoch) file name
fn extract_digits_from_file_name(path: &PathBuf) -> i64 {
    let file_name = path.file_name().unwrap().to_str().unwrap();
    file_name
        .split('-')
        .nth(1)
        .unwrap()
        .split('-')
        .next()
        .unwrap()
        .parse::<i64>()
        .unwrap()
}

async fn insert_accounts(
    pool: &DbPool,
    accounts: HashSet<String>,
) -> Result<(), edgedb_tokio::Error> {
    let accounts_vec: Vec<String> = accounts.into_iter().collect();

    for chunk in accounts_vec.chunks(ACCOUNTS_BATCH_SIZE) {
        let mut futures = Vec::new();

        for account in chunk {
            let account = account.clone();
            let future = pool.execute(
                "insert Account {public_key := <str>$0} unless conflict;".to_string(),
                (account,),
            );
            futures.push(future);
        }

        try_join_all(futures).await?;
    }

    Ok(())
}

/// These should really all be u64 but the conversion to EdgeDB requires i64
/// For some reason parsing `as_number` doesn't work
fn to_i64(value: &Value) -> Option<i64> {
    value.as_str().and_then(|s| s.parse().ok())
}

fn to_decimal(value: &Value) -> Option<BigDecimal> {
    match value.get_type() {
        JsonType::Number => {
            if let Some(num_str) = value.as_str() {
                // sonic_rs stores numbers as strings internally
                if num_str.contains('.') {
                    // It's a floating-point number
                    BigDecimal::from_str(num_str).ok()
                } else {
                    // It's an integer
                    num_str.parse::<i64>().ok().map(BigDecimal::from)
                }
            } else {
                None
            }
        }
        JsonType::String => value.as_str().and_then(|s| BigDecimal::from_str(s).ok()),
        _ => None,
    }
}

async fn to_json(path: &PathBuf) -> io::Result<Value> {
    let file_contents = fs::read(path)?;

    match sonic_rs::from_slice(&file_contents) {
        Ok(value) => Ok(value),
        Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
    }
}

fn account_link(public_key: &Value) -> String {
    format!(
        "(select Account filter .public_key = '{}')",
        public_key.as_str().unwrap()
    )
}
