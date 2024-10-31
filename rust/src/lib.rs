use bigdecimal::BigDecimal;
use db::{DbPool, MAX_CONNECTIONS};
use sonic_rs::{JsonType, JsonValueTrait, Value};
use std::{collections::HashSet, str::FromStr};

pub mod blocks;
mod db;
mod files;
pub mod staking;
mod stats;

const ACCOUNTS_BATCH_SIZE: usize = MAX_CONNECTIONS / 3;

fn to_titlecase(s: &str) -> String {
    s.chars()
        .next()
        .map(|c| c.to_ascii_uppercase())
        .into_iter()
        .chain(s.chars().skip(1))
        .collect()
}

async fn insert_accounts(
    pool: &DbPool,
    accounts: HashSet<String>,
) -> Result<(), edgedb_tokio::Error> {
    // Process all accounts in a single transaction with conflict handling
    for chunk in accounts
        .into_iter()
        .collect::<Vec<String>>()
        .chunks(ACCOUNTS_BATCH_SIZE)
    {
        let query = format!(
            "FOR account_pk IN {{{}}}
             UNION (
                INSERT Account {{
                    public_key := account_pk
                }} UNLESS CONFLICT
             )",
            chunk
                .iter()
                .map(|a| format!("'{}'", a))
                .collect::<Vec<_>>()
                .join(", ")
        );

        pool.execute(query, ()).await?;
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

fn account_link(public_key: &Value) -> String {
    format!(
        "(select Account filter .public_key = '{}')",
        public_key.as_str().unwrap()
    )
}
