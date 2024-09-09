use bigdecimal::BigDecimal;
use edgedb_tokio::{Builder, Client, RetryCondition, RetryOptions};
use rayon::prelude::*;
use sonic_rs::{JsonType, JsonValueTrait, Value};
use std::{cmp::Ordering, collections::HashSet, fs, io, path::PathBuf, str::FromStr, sync::Arc};

pub mod blocks;
pub mod staking;

/// Get (and sort) file paths for a given directory
fn get_file_paths(dir: &str) -> Result<Vec<PathBuf>, io::Error> {
    // Read directory entries
    let entries = fs::read_dir(dir)?;

    // Collect and filter entries in parallel
    let paths: Vec<PathBuf> = entries
        .par_bridge()
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                let path = e.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                    Some(path)
                } else {
                    None
                }
            })
        })
        .collect();

    // Parallel sorting
    let mut sorted_paths = paths;
    sorted_paths.par_sort_unstable_by(|a, b| {
        natural_sort(
            a.file_name().unwrap().to_str().unwrap(),
            b.file_name().unwrap().to_str().unwrap(),
        )
    });

    Ok(sorted_paths)
}

fn natural_sort(a: &str, b: &str) -> Ordering {
    let mut a_parts = a.split(|c: char| !c.is_numeric());
    let mut b_parts = b.split(|c: char| !c.is_numeric());

    loop {
        match (a_parts.next(), b_parts.next()) {
            (Some(a_part), Some(b_part)) => {
                if let (Ok(a_num), Ok(b_num)) = (a_part.parse::<u32>(), b_part.parse::<u32>()) {
                    match a_num.cmp(&b_num) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                }
                match a_part.cmp(b_part) {
                    Ordering::Equal => continue,
                    other => return other,
                }
            }
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
        }
    }
}

/// Get a [database][Client] connection pool
async fn get_db(num_connections: usize) -> Result<Arc<Client>, edgedb_tokio::Error> {
    let db_builder = Client::new(
        &Builder::new()
            .max_concurrency(num_connections)
            .build_env()
            .await?,
    );

    let retry_opts = RetryOptions::default().with_rule(
        RetryCondition::TransactionConflict,
        // No. of retries
        10,
        // Retry immediately instead of default with increasing backoff
        |_| std::time::Duration::from_secs(3),
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
