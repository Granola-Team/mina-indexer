use crate::{files::process_files, get_db_connection, insert_accounts, to_decimal, to_i64};
use anyhow::Result;
use sonic_rs::{Array, JsonContainerTrait, JsonValueTrait, Value};
use std::collections::HashSet;
use tracing::{debug, info};

const BATCH_SIZE: usize = 10_000;

pub async fn run(staking_ledgers_dir: String) -> Result<()> {
    process_files(&staking_ledgers_dir, |json, hash, number| async move {
        process_ledger(json.as_array().expect("ledger").to_owned(), hash, number).await
    })
    .await
}

const INSERT_EPOCH: &str = "
    INSERT INTO staking_epochs (hash, epoch)
    VALUES (?, ?)
    ON CONFLICT (hash, epoch) DO NOTHING
";

const INSERT_LEDGER_BATCH: &str = "
    INSERT INTO staking_ledgers (
        epoch_hash, epoch_number, source, balance, target, token,
        nonce, receipt_chain_hash, voting_for
    )
    SELECT * FROM (VALUES %s)
    RETURNING id
";

const INSERT_TIMING_BATCH: &str = "
    INSERT INTO staking_timing (
        ledger_id, initial_minimum_balance, cliff_time,
        cliff_amount, vesting_period, vesting_increment
    )
    VALUES %s
";

struct BatchData {
    ledger_values: Vec<String>,
    timing_values: Vec<(String, i64, String, i64, String)>,
}

impl BatchData {
    fn with_capacity(size: usize) -> Self {
        Self {
            ledger_values: Vec::with_capacity(size),
            timing_values: Vec::with_capacity(size),
        }
    }

    fn clear(&mut self) {
        self.ledger_values.clear();
        self.timing_values.clear();
    }
}

async fn process_ledger(json: Array, ledger_hash: String, epoch: i64) -> Result<(), duckdb::Error> {
    info!("Processing ledger {} at epoch {}", ledger_hash, epoch);
    let accounts = extract_accounts(&json);
    debug!("Extracted accounts");

    insert_accounts(accounts)?;

    // Create DB connection at start and keep it for entire function
    let mut db = get_db_connection()?;
    db.execute(INSERT_EPOCH, [&ledger_hash, &epoch.to_string()])?;
    let tx = db.transaction()?;

    let mut batch = BatchData::with_capacity(BATCH_SIZE);
    let total_records = json.len();

    for chunk_start in (0..total_records).step_by(BATCH_SIZE) {
        let chunk_end = (chunk_start + BATCH_SIZE).min(total_records);
        batch.clear();

        // Process chunk of records
        for activity in &json[chunk_start..chunk_end] {
            let source = activity["pk"].as_str().expect("pk");
            let delegate = activity["delegate"].as_str().unwrap_or(source);

            let ledger_value = format!(
                "('{}', {}, '{}', '{}', '{}', {}, {}, '{}', '{}')",
                ledger_hash,
                epoch,
                source,
                to_decimal(&activity["balance"]).expect("balance"),
                delegate,
                to_i64(&activity["token"]).unwrap_or(0),
                to_i64(&activity["nonce"]).unwrap_or(0),
                activity["receipt_chain_hash"].as_str().unwrap_or_default(),
                activity["voting_for"].as_str().unwrap_or_default()
            );
            batch.ledger_values.push(ledger_value);

            if let Some(timing) = activity["timing"].as_object() {
                batch.timing_values.push((
                    to_decimal(&timing["initial_minimum_balance"]).expect("initial_minimum_balance").to_string(),
                    to_i64(&timing["cliff_time"]).expect("cliff_time"),
                    to_decimal(&timing["cliff_amount"]).expect("cliff_amount").to_string(),
                    to_i64(&timing["vesting_period"]).expect("vesting_period"),
                    to_decimal(&timing["vesting_increment"]).expect("vesting_increment").to_string(),
                ));
            }
        }

        // Bulk insert ledger entries
        if !batch.ledger_values.is_empty() {
            let ledger_query = INSERT_LEDGER_BATCH.replace("%s", &batch.ledger_values.join(","));
            let ledger_ids: Vec<i64> = tx
                .prepare(&ledger_query)?
                .query_map([], |row| row.get::<_, i64>(0))?
                .filter_map(Result::ok)
                .collect();

            // Bulk insert timing data
            if !batch.timing_values.is_empty() {
                let timing_values: Vec<String> = batch
                    .timing_values
                    .iter()
                    .zip(ledger_ids.iter())
                    .map(|((imb, ct, ca, vp, vi), id)| format!("({}, '{}', {}, '{}', {}, '{}')", id, imb, ct, ca, vp, vi))
                    .collect();

                let timing_query = INSERT_TIMING_BATCH.replace("%s", &timing_values.join(","));
                tx.execute(&timing_query, [])?;
            }
        }

        tx.commit()?;
    }

    Ok(())
}

fn extract_accounts(json_array: &[Value]) -> HashSet<String> {
    json_array
        .iter()
        .flat_map(|obj| {
            let pk = obj.get("pk").and_then(Value::as_str);
            let delegate = obj.get("delegate").and_then(Value::as_str);
            pk.into_iter().chain(delegate).map(String::from)
        })
        .collect()
}
