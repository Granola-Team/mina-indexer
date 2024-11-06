use crate::db::DbPool;
use rayon::{
    iter::{IntoParallelIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use sonic_rs::Value;
use std::{
    collections::VecDeque,
    future::Future,
    io,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{io::AsyncReadExt, sync::Mutex, task::JoinHandle};
use walkdir::WalkDir;

const FILE_PREFIX: &str = "mainnet-";
pub const CHUNK_SIZE: usize = 100;
const BUFFER_SIZE: usize = 16 * 1024; // 16KB buffer

struct ChunkProcessor<F> {
    queue: Mutex<VecDeque<Vec<PathBuf>>>,
    processed_chunks: AtomicUsize,
    total_chunks: usize,
    start_time: Instant,
    processor: F,
}

impl<F> ChunkProcessor<F>
where
    F: Fn(Arc<DbPool>, Value, String, i64) -> Pin<Box<dyn Future<Output = Result<(), edgedb_tokio::Error>> + Send>> + Send + Sync + 'static,
{
    fn new(paths: Vec<PathBuf>, processor: F) -> Self {
        let total_chunks = paths.len().div_ceil(CHUNK_SIZE);
        let chunks: VecDeque<Vec<PathBuf>> = paths.chunks(CHUNK_SIZE).map(|c| c.to_vec()).collect();

        Self {
            queue: Mutex::new(chunks),
            processed_chunks: AtomicUsize::new(0),
            total_chunks,
            start_time: Instant::now(),
            processor,
        }
    }

    fn format_duration(duration: Duration) -> String {
        let total_secs = duration.as_secs();
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;

        if hours > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}m", mins)
        }
    }

    fn get_stats(&self, pool: &DbPool) -> String {
        let elapsed = self.start_time.elapsed();
        let processed = self.processed_chunks.load(Ordering::Relaxed);

        let percentage = (processed as f64 / self.total_chunks as f64 * 100.0).round() as u32;

        let remaining = if processed > 0 {
            let avg_time_per_chunk = elapsed.div_f64(processed as f64);
            avg_time_per_chunk.mul_f64((self.total_chunks - processed) as f64)
        } else {
            Duration::ZERO
        };

        format!(
            "Progress: {}/{} chunks ({}%), elapsed: {}, remaining: {}{}",
            processed,
            self.total_chunks,
            percentage,
            Self::format_duration(elapsed),
            Self::format_duration(remaining),
            pool.get_pool_stats()
        )
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

                    let json = sonic_rs::from_slice(&contents).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

                    let hash = extract_hash_from_file_name(&path);
                    let number = extract_digits_from_file_name(&path);

                    match (processor)(pool, json, hash.to_owned(), number).await {
                        Ok(_) => Ok(()),
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

        self.processed_chunks.fetch_add(1, Ordering::SeqCst);

        let processed = self.processed_chunks.load(Ordering::SeqCst);
        if processed % 10 == 0 {
            println!("{}", self.get_stats(pool));
        }

        Some(())
    }
}

#[inline]
/// Get [file paths][PathBuf] from `dir`
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
                && e.file_name().to_str().map_or(false, |name| name.starts_with(FILE_PREFIX))
        })
        .for_each(|e| paths.push(e.into_path()));

    // Sort by number
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

fn spawn_workers<F>(initial_workers: usize, max_workers: usize, processor: Arc<ChunkProcessor<F>>, pool: Arc<DbPool>) -> Vec<JoinHandle<()>>
where
    F: Fn(Arc<DbPool>, Value, String, i64) -> Pin<Box<dyn Future<Output = Result<(), edgedb_tokio::Error>> + Send>> + Send + Sync + 'static,
{
    let mut handles = Vec::with_capacity(max_workers - initial_workers);

    for _ in initial_workers..max_workers {
        let processor = Arc::clone(&processor);
        let pool = Arc::clone(&pool);

        let handle = tokio::spawn(async move { while processor.process_next_chunk(&pool).await.is_some() {} });

        handles.push(handle);
    }

    handles
}

pub async fn process_files<F>(dir: &str, pool: Arc<DbPool>, processor_fn: F) -> anyhow::Result<()>
where
    F: Fn(Arc<DbPool>, Value, String, i64) -> Pin<Box<dyn Future<Output = Result<(), edgedb_tokio::Error>> + Send>> + Send + Sync + 'static,
{
    println!("Processing files in: {}", dir);
    let paths = get_file_paths(dir)?;
    let processor = ChunkProcessor::new(paths, processor_fn);
    let processor = Arc::new(processor);

    let initial_workers = 2;
    let max_workers = num_cpus::get() * 2;

    // Start with few workers initially, then scale up
    let mut handles = spawn_workers(0, initial_workers, Arc::clone(&processor), Arc::clone(&pool));

    // After 4 minutes, add more workers
    tokio::time::sleep(Duration::from_secs(240)).await;
    println!("Adding more workers");

    handles.extend(spawn_workers(initial_workers, max_workers, Arc::clone(&processor), Arc::clone(&pool)));

    for handle in handles {
        if let Err(e) = handle.await {
            eprintln!("Task failed: {}", e);
        }
    }

    Ok(())
}

/// Extract the hash part from a Mina block or staking ledger file name
#[inline]
fn extract_hash_from_file_name(path: &Path) -> &str {
    path.file_name()
        .and_then(|n| n.to_str())
        .and_then(|s| s.split('-').nth(2))
        .and_then(|s| s.split('.').next())
        .expect("Invalid file name format")
}

/// Extract the digits part from a Mina block (the height) or staking ledger (the epoch) file name
#[inline]
fn extract_digits_from_file_name(path: &Path) -> i64 {
    path.file_name()
        .and_then(|n| n.to_str())
        .and_then(|s| s.split('-').nth(1))
        .and_then(|s| s.split('-').next())
        .and_then(|s| s.parse().ok())
        .expect("Invalid file name format")
}
