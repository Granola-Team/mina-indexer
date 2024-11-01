use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rayon::slice::ParallelSliceMut;
use sonic_rs::Value;
use std::collections::VecDeque;
use std::future::Future;
use std::io;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use walkdir::WalkDir;

use crate::db::DbPool;
use crate::stats::ProcessingStats;

const BLOCK_FILE_PREFIX: &str = "mainnet-";
pub const CHUNK_SIZE: usize = 100;
const BUFFER_SIZE: usize = 16 * 1024; // 16KB buffer

struct ChunkProcessor<F> {
    queue: Mutex<VecDeque<Vec<PathBuf>>>,
    stats: Arc<ProcessingStats>,
    processor: F,
}

impl<F> ChunkProcessor<F>
where
    F: Fn(
            Arc<DbPool>,
            Value,
            String,
            i64,
        ) -> Pin<Box<dyn Future<Output = Result<(), edgedb_tokio::Error>> + Send>>
        + Send
        + Sync
        + 'static,
{
    fn new(paths: Vec<PathBuf>, processor: F) -> Self {
        let total_chunks = (paths.len() + CHUNK_SIZE - 1) / CHUNK_SIZE;
        let chunks: VecDeque<Vec<PathBuf>> = paths.chunks(CHUNK_SIZE).map(|c| c.to_vec()).collect();

        Self {
            queue: Mutex::new(chunks),
            stats: Arc::new(ProcessingStats::new(total_chunks)),
            processor,
        }
    }

    async fn process_next_chunk(&self, pool: &Arc<DbPool>) -> Option<()> {
        let chunk = {
            let mut queue = self.queue.lock().await;
            queue.pop_front()?
        };

        let futures = chunk
            .into_par_iter()
            .map(|path| {
                let pool = Arc::clone(pool);
                let processor = &self.processor;

                async move {
                    let file = tokio::fs::File::open(&path).await?;
                    let metadata = file.metadata().await?;

                    let mut reader = tokio::io::BufReader::with_capacity(BUFFER_SIZE, file);
                    let mut contents = Vec::with_capacity(metadata.len() as usize);
                    reader.read_to_end(&mut contents).await?;

                    let json = sonic_rs::from_slice(&contents)
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

                    let hash = extract_hash_from_file_name(&path);
                    let number = extract_digits_from_file_name(&path);

                    match (processor)(pool, json, hash.clone(), number).await {
                        Ok(_) => {
                            //println!("Hash {} (height {}) processed", hash, number);
                            Ok(())
                        }
                        Err(e) => {
                            println!("Error processing {} ({})", hash, number);
                            Err(io::Error::new(io::ErrorKind::Other, e.to_string()))
                        }
                    }
                }
            })
            .collect::<Vec<_>>();

        for future in futures {
            if let Err(e) = future.await {
                eprintln!("{}", e);
            }
        }

        self.stats.update();

        let processed = self.stats.processed_count();
        if processed % 10 == 0 {
            println!("{}", self.stats.get_stats());
        }

        Some(())
    }
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

fn spawn_workers<F>(
    initial_workers: usize,
    max_workers: usize,
    processor: Arc<ChunkProcessor<F>>,
    pool: Arc<DbPool>,
) -> Vec<JoinHandle<()>>
where
    F: Fn(
            Arc<DbPool>,
            Value,
            String,
            i64,
        ) -> Pin<Box<dyn Future<Output = Result<(), edgedb_tokio::Error>> + Send>>
        + Send
        + Sync
        + 'static,
{
    let mut handles = Vec::with_capacity(max_workers - initial_workers);

    for _ in initial_workers..max_workers {
        let processor = Arc::clone(&processor);
        let pool = Arc::clone(&pool);

        let handle = tokio::spawn(async move {
            while let Some(_) = processor.process_next_chunk(&pool).await {}
        });

        handles.push(handle);
    }

    handles
}

pub async fn process_files<F>(dir: &str, pool: Arc<DbPool>, processor_fn: F) -> anyhow::Result<()>
where
    F: Fn(
            Arc<DbPool>,
            Value,
            String,
            i64,
        ) -> Pin<Box<dyn Future<Output = Result<(), edgedb_tokio::Error>> + Send>>
        + Send
        + Sync
        + 'static,
{
    println!("Processing files in: {}", dir);
    let paths = get_file_paths(dir)?;
    let processor = Arc::new(ChunkProcessor::new(paths, processor_fn));

    let initial_workers = 2;
    let max_workers = num_cpus::get() * 2;

    // Start with few workers initially, then scale up
    let mut handles = spawn_workers(
        0,
        initial_workers,
        Arc::clone(&processor),
        Arc::clone(&pool),
    );

    // After 4 minutes, add more workers
    tokio::time::sleep(Duration::from_secs(240)).await;
    println!("Adding more workers");

    handles.extend(spawn_workers(
        initial_workers,
        max_workers,
        Arc::clone(&processor),
        Arc::clone(&pool),
    ));

    for handle in handles {
        if let Err(e) = handle.await {
            eprintln!("Task failed: {}", e);
        }
    }

    Ok(())
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
