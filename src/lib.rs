use bigdecimal::BigDecimal;
use edgedb_tokio::{Builder, Client, RetryCondition, RetryOptions};
use serde_json::Value;
use std::{collections::HashSet, fs, path::PathBuf, sync::Arc};
use tokio::{
    fs::File,
    io::{self, AsyncReadExt, BufReader},
};

pub mod blocks;
pub mod staking;

/// Get (and sort) file paths for a given directory
fn get_file_paths(dir: &str) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut paths = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().map_or(false, |ext| ext == "json"))
        .collect::<Vec<_>>();

    // Sort by filename
    paths.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    Ok(paths)
}

/// Get a [database][Client] connection pool
async fn get_db(num_connections: usize) -> Result<Arc<Client>, edgedb_tokio::Error> {
    let db_builder = Client::new(
        &Builder::new()
            .max_concurrency(num_connections)
            .build_env()
            .await?,
    );

    let retry_opts = RetryOptions::default().with_rule::<u8>(
        RetryCondition::TransactionConflict,
        // No. of retries
        3,
        // Retry immediately instead of default with increasing backoff
        |_| std::time::Duration::from_millis(500),
    );

    Ok(Arc::new(db_builder.with_retry_options(retry_opts)))
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
fn extract_hash_from_file_name(path: &PathBuf) -> &str {
    let file_name = path.file_name().unwrap().to_str().unwrap();
    file_name
        .split('-')
        .nth(2)
        .unwrap()
        .split('.')
        .next()
        .unwrap()
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
    db: &Arc<Client>,
    accounts: HashSet<String>,
) -> Result<(), edgedb_tokio::Error> {
    for account in accounts {
        db.execute(
            "insert Account {public_key := <str>$0} unless conflict;",
            &(account,),
        )
        .await?;
    }

    Ok(())
}

/// These should really all be u64 but the conversion to EdgeDB requires i64
fn to_i64(value: &Value) -> Option<i64> {
    value.as_str().and_then(|s| s.parse().ok())
}

fn to_decimal(value: &Value) -> Option<BigDecimal> {
    match value {
        Value::Number(num) => {
            if num.is_i64() {
                num.as_i64().map(BigDecimal::from)
            } else if num.is_f64() {
                num.as_f64().and_then(|n| BigDecimal::try_from(n).ok())
            } else {
                None
            }
        }
        Value::String(s) => s.parse::<BigDecimal>().ok(),
        _ => None,
    }
}

async fn to_json(path: &PathBuf) -> io::Result<Value> {
    let file = File::open(path).await?;
    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).await?;

    // First, try to parse directly from the buffer
    match serde_json::from_slice(&buffer) {
        Ok(value) => Ok(value),
        Err(e) => {
            // It is too slow to try to use `String::from_utf8_lossy(&buffer)`
            // So, just throw an `InvalidData`
            Err(io::Error::new(io::ErrorKind::InvalidData, e))
        }
    }
}
