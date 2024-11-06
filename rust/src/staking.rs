use crate::{
    account_link,
    db::DbPool,
    files::{process_files, CHUNK_SIZE},
    insert_accounts, to_decimal, to_i64,
};
use futures::future::try_join_all;
use sonic_rs::{Array, JsonContainerTrait, JsonValueTrait, Value};
use std::{collections::HashSet, sync::Arc};
use tracing::{debug, info};

/// Ingest staking ledger files (JSON) from `staking_ledgers_dir` into the database
pub async fn run(staking_ledgers_dir: &str) -> anyhow::Result<()> {
    let pool = Arc::new(DbPool::new(Some("trunk")).await?);
    process_files(staking_ledgers_dir, pool, |pool, json, hash, number| {
        Box::pin(process_ledger(pool, json.as_array().expect("ledger").to_owned(), hash, number))
    })
    .await
}

const INSERT_EPOCH: &str = "insert StakingEpoch {
    hash := <str>$0,
    epoch := <int64>$1
} unless conflict;";

/// Process ledger
async fn process_ledger(pool: Arc<DbPool>, json: Array, ledger_hash: String, epoch: i64) -> Result<(), edgedb_tokio::Error> {
    info!("Processing ledger {} at epoch {}", ledger_hash, epoch);
    let accounts = extract_accounts(&json);
    debug!("Extracted accounts");

    let insert_ledger = |source: &str, target: &str| {
        format!(
            "insert StakingLedger {{
                epoch := assert_single((select StakingEpoch filter .epoch = {} and .hash = '{}')),
                source := {},
                balance := <decimal>$0,
                target := {},
                token := <int64>$1,
                nonce := <optional int64>$2,
                receipt_chain_hash := <str>$3,
                voting_for := <str>$4
            }}",
            epoch, ledger_hash, source, target
        )
    };

    let insert_timing = |source: &str, target: &str| {
        format!(
            "with ledger := (
            {}
        )
        insert StakingTiming {{
            ledger := ledger,
            initial_minimum_balance := <decimal>$5,
            cliff_time := <int64>$6,
            cliff_amount := <decimal>$7,
            vesting_period := <int64>$8,
            vesting_increment := <decimal>$9
        }};",
            insert_ledger(source, target)
        )
    };

    let epoch_params = (ledger_hash.clone(), epoch);
    let (_, _) = tokio::try_join!(insert_accounts(&pool, accounts), pool.execute(INSERT_EPOCH, &epoch_params))?;

    for chunk in json.chunks(CHUNK_SIZE) {
        let mut futures = Vec::new();

        for activity in chunk {
            let pool = Arc::clone(&pool);

            // Extract account links for formatting into query
            let source = account_link(&activity["pk"]);
            let delegate = &activity["delegate"];
            // You must have a delegate and if it's not stated, you are delegating to yourself
            let target = if delegate.as_str().is_some() {
                account_link(delegate)
            } else {
                source.clone()
            };
            let ledger_query = insert_ledger(&source, &target);
            let timing_query = insert_timing(&source, &target);

            // Extract remaining values before the async move
            let balance = to_decimal(&activity["balance"]);
            let token = to_i64(&activity["token"]).unwrap_or(0);
            let nonce = to_i64(&activity["nonce"]);
            let receipt_chain_hash = activity["receipt_chain_hash"].as_str().unwrap_or_default().to_string();
            let voting_for = activity["voting_for"].as_str().unwrap_or_default().to_string();

            let base_params = (balance, token, nonce, receipt_chain_hash, voting_for);

            let timing_data = activity["timing"].as_object().map(|timing| {
                (
                    to_decimal(&timing["initial_minimum_balance"]),
                    to_i64(&timing["cliff_time"]),
                    to_decimal(&timing["cliff_amount"]),
                    to_i64(&timing["vesting_period"]),
                    to_decimal(&timing["vesting_increment"]),
                )
            });

            let future = async move {
                if let Some(timing_values) = timing_data {
                    let timing_params = (
                        base_params.0,
                        base_params.1,
                        base_params.2,
                        base_params.3,
                        base_params.4,
                        timing_values.0,
                        timing_values.1,
                        timing_values.2,
                        timing_values.3,
                        timing_values.4,
                    );
                    pool.execute(&timing_query, &timing_params).await
                } else {
                    pool.execute(&ledger_query, &base_params).await
                }
            };

            futures.push(future);
        }

        try_join_all(futures).await?;
    }

    Ok(())
}

/// Extract a [list][HashSet] of accounts (public keys)
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
