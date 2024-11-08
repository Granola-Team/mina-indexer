use anyhow::Result;
use bigdecimal::BigDecimal;
use duckdb::{params_from_iter, Connection};
use sonic_rs::{JsonType, JsonValueTrait, Value};
use std::{collections::HashSet, future::Future, str::FromStr};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

pub(crate) mod blockchain_tree;
pub mod blocks;
pub mod constants;
pub mod files;
pub mod staking;
pub mod stream;
pub mod utility;

const ACCOUNTS_BATCH_SIZE: usize = 100;

const DB_FILE: &str = "mina.db";

pub fn get_db_connection() -> Result<Connection, duckdb::Error> {
    let conn = Connection::open(DB_FILE)?;
    //conn.execute_batch(insert_sql!());

    Ok(conn)
}

pub fn check_or_create_db_schema() -> Result<(), duckdb::Error> {
    let db = get_db_connection()?;

    let schema = include_str!("../../db/schema.sql");
    db.execute_batch(schema)?;

    Ok(())
}

fn insert_accounts(accounts: HashSet<String>) -> Result<(), duckdb::Error> {
    for chunk in accounts.into_iter().collect::<Vec<String>>().chunks(ACCOUNTS_BATCH_SIZE) {
        let placeholders = chunk.iter().enumerate().map(|(i, _)| format!("(?{})", i + 1)).collect::<Vec<_>>().join(",");

        let query = format!(
            "INSERT INTO accounts (public_key)
             SELECT v.value FROM (VALUES {}) AS v(value)
             WHERE NOT EXISTS (
                 SELECT 1 FROM accounts
                 WHERE public_key = v.value
             )",
            placeholders
        );

        let params: Vec<&str> = chunk.iter().map(|s| s.as_str()).collect();
        get_db_connection()?.execute(&query, params_from_iter(&params))?;
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

pub async fn start<F, Fut>(file_path: &str, function: F) -> Result<()>
where
    F: FnOnce(String) -> Fut + Send + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    // log to stdout
    let subscriber = FmtSubscriber::builder().with_max_level(Level::INFO).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    check_or_create_db_schema()?;

    let file_path = file_path.to_string();

    match tokio::spawn(async move {
        match function(file_path).await {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("Error in processing: {}", e);
                Err(e)
            }
        }
    })
    .await
    {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Task join error: {}", e);
            Err(anyhow::anyhow!("Task join error: {}", e))
        }
    }
}
