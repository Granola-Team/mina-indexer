use crate::{
    files::{extract_digits_from_file_name, extract_hash_from_file_name, get_file_paths},
    get_db_connection,
};
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::{thread, time::Duration};
use tracing::{error, info, warn};

const MAX_RETRIES: u32 = 6;
const RETRY_DELAY_MS: u64 = 5000;
const BATCH_SIZE: usize = 3_000;
const SUB_BATCH_SIZE: usize = 600;

struct SqlStatements {
    statements: Vec<&'static str>,
}

impl SqlStatements {
    fn new() -> Self {
        Self {
            statements: vec![
                include_str!("../../db/create_temp_views.sql"),
                include_str!("../../db/insert_accounts.sql"),
                include_str!("../../db/insert_blocks.sql"),
                include_str!("../../db/insert_blockchain_states.sql"),
                include_str!("../../db/insert_staged_ledger_hashes.sql"),
                include_str!("../../db/insert_snark_jobs.sql"),
                include_str!("../../db/insert_user_commands.sql"),
                include_str!("../../db/insert_internal_commands.sql"),
                include_str!("../../db/insert_epoch_data.sql"),
            ],
        }
    }
}

struct ChunkProcessor {
    sql_statements: SqlStatements,
}

impl ChunkProcessor {
    fn new() -> Self {
        Self {
            sql_statements: SqlStatements::new(),
        }
    }

    fn process_chunk(&self, chunk: &[std::path::PathBuf], retry_count: u32) -> Result<()> {
        info!("Processing chunk of {} files (attempt {})", chunk.len(), retry_count + 1);

        let result = (|| -> Result<()> {
            let mut db = get_db_connection().context("Failed to get DB connection")?;
            let tx = db.transaction().context("Failed to start transaction")?;

            // Create raw_blocks table first
            tx.execute_batch(include_str!("../../db/create_temp_table.sql"))?;

            // Pre-process file data in parallel
            let prepared_data: Vec<_> = chunk
                .par_chunks(SUB_BATCH_SIZE)
                .flat_map(|sub_chunk| {
                    sub_chunk
                        .iter()
                        .map(|path| {
                            let file_path = path.to_str().unwrap_or_default();
                            let block_hash = extract_hash_from_file_name(path);
                            let height = extract_digits_from_file_name(path);

                            (block_hash, height, file_path.to_string())
                        })
                        .collect::<Vec<_>>()
                })
                .collect();

            // Create temporary tables and insert raw data
            let mut stmt = tx.prepare("INSERT INTO raw_blocks SELECT ? AS hash, ? AS height, json FROM read_json(?) AS json")?;

            for (block_hash, height, file_path) in prepared_data {
                match stmt.execute([block_hash, &height.to_string(), &file_path]) {
                    Ok(_) => info!("Loaded block {} at height {}", block_hash, height),
                    Err(e) => {
                        error!("Error loading block {} at height {}: {}. Skipping this block.", block_hash, height, e);
                        continue; // Skip problematic blocks instead of failing the entire chunk
                    }
                }
            }

            // Execute remaining SQL statements
            for statement in &self.sql_statements.statements {
                if let Err(e) = tx.execute_batch(statement) {
                    error!("Error executing SQL statement {}: {}", statement, e);
                    return Err(e.into());
                }
            }

            tx.commit()?;
            Ok(())
        })();

        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                if retry_count < MAX_RETRIES {
                    warn!("Chunk processing failed: {}. Retrying in {} ms...", e, RETRY_DELAY_MS);
                    thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
                    self.process_chunk(chunk, retry_count + 1)
                } else {
                    error!("Failed to process chunk after {} retries", MAX_RETRIES);
                    Err(e)
                }
            }
        }
    }
}

pub fn run(blocks_dir: &str) -> Result<()> {
    let paths = get_file_paths(blocks_dir)?;
    let processor = ChunkProcessor::new();
    let mut failed_chunks = Vec::new();

    for (chunk_idx, chunk) in paths.chunks(BATCH_SIZE).enumerate() {
        info!("Processing chunk {}", chunk_idx);

        if let Err(e) = processor.process_chunk(chunk, 0) {
            error!("Failed to process chunk {}: {}", chunk_idx, e);
            failed_chunks.push(chunk.to_vec());
        }
    }

    if !failed_chunks.is_empty() {
        error!("Failed to process {} chunks", failed_chunks.len());
    }

    Ok(())
}
