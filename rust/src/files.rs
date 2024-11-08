use anyhow::Result;
use rayon::prelude::*;
use sonic_rs::Value;
use std::{
    future::Future,
    io::{self, BufReader, Read},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tracing::info;
use walkdir::WalkDir;

const FILE_PREFIX: &str = "mainnet-";

#[inline]
/// Get [file paths][PathBuf] from `dir`
fn get_file_paths(dir: &str) -> Result<Vec<PathBuf>, io::Error> {
    let mut paths: Vec<PathBuf> = Vec::new();

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

pub async fn process_files<F, Fut>(dir: &str, processor_fn: F) -> Result<()>
where
    F: Fn(Value, String, i64) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), duckdb::Error>> + Send + 'static,
{
    info!("Processing files in: {}", dir);
    let paths = get_file_paths(dir)?;
    let total_files = paths.len();
    let processed = Arc::new(AtomicUsize::new(0));
    let processor_fn = Arc::new(processor_fn);

    let chunks: Vec<_> = paths.chunks(5).map(|c| c.to_vec()).collect();

    for chunk in chunks {
        let futures = chunk
            .par_iter()
            .map(|path| {
                let file = std::fs::File::open(path)?;
                let metadata = file.metadata()?;

                // Use streaming for large files
                let mut reader = if metadata.len() > 100_000_000 {
                    BufReader::with_capacity(16 * 1024 * 1024, file) // 16MB buffer for large files
                } else {
                    BufReader::with_capacity(1024 * 1024, file) // 1MB buffer for regular files
                };

                let contents = if metadata.len() > 100_000_000 {
                    // Stream large files in chunks
                    let mut buffer = Vec::new();
                    let mut chunk = vec![0; 1024 * 1024]; // 1MB chunks
                    loop {
                        match reader.read(&mut chunk)? {
                            0 => break,
                            n => buffer.extend_from_slice(&chunk[..n]),
                        }
                    }
                    buffer
                } else {
                    // Read smaller files directly
                    let mut buffer = Vec::with_capacity(metadata.len() as usize);
                    reader.read_to_end(&mut buffer)?;
                    buffer
                };

                // Parse JSON with validation
                let json = match sonic_rs::from_slice::<Value>(&contents) {
                    Ok(json) => json,
                    Err(e) => return Err(anyhow::anyhow!("JSON parse error in {}: {}", path.display(), e)),
                };

                let hash = extract_hash_from_file_name(path);
                let number = extract_digits_from_file_name(path);

                Ok::<_, anyhow::Error>((json, hash.to_owned(), number))
            })
            .collect::<Result<Vec<_>>>()?;

        let mut handles = Vec::with_capacity(futures.len());

        for (json, hash, number) in futures {
            let processor_fn = Arc::clone(&processor_fn);
            let processed = Arc::clone(&processed);

            let handle = tokio::spawn(async move {
                let result = (processor_fn)(json, hash, number).await;
                let count = processed.fetch_add(1, Ordering::SeqCst) + 1;
                if count % 100 == 0 {
                    println!(
                        "Progress: {}/{} files ({}%)",
                        count,
                        total_files,
                        (count as f64 / total_files as f64 * 100.0) as u32
                    );
                }
                result
            });
            handles.push(handle);
        }

        // Wait for all handles in current chunk
        for handle in handles {
            handle.await??;
        }
    }

    let final_count = processed.load(Ordering::SeqCst);
    info!("Processing complete. Processed {}/{} files", final_count, total_files);

    if final_count != total_files {
        anyhow::bail!("Not all files were processed: {}/{}", final_count, total_files);
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
