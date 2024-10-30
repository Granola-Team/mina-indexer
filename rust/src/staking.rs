use bigdecimal::BigDecimal;
use futures::future::try_join_all;
use sonic_rs::{JsonContainerTrait, JsonValueTrait, Value};
use std::{collections::HashSet, sync::Arc};

use crate::{account_link, chunk_size, db::DbPool, insert_accounts, process_files, to_decimal};

/// Ingest staking ledger files (JSON) into the database
pub async fn run(staking_ledgers_dir: &str) -> anyhow::Result<()> {
    let pool = Arc::new(DbPool::new().await?);
    process_files(staking_ledgers_dir, pool, process_ledger).await
}

async fn process_ledger(
    pool: Arc<DbPool>,
    json: Value,
    ledger_hash: String,
    epoch: i64,
) -> Result<(), edgedb_tokio::Error> {
    let json = json.as_array().unwrap();
    let accounts = extract_accounts(json);
    let ledger_hash = Arc::new(ledger_hash);

    // Run account insertion and epoch creation concurrently
    let query = "insert StakingEpoch {
            hash := <str>$0,
            epoch := <int64>$1
        } unless conflict;"
        .to_string();

    let ledger_hash_str = ledger_hash.as_str().to_string();
    let (_, _) = tokio::try_join!(
        insert_accounts(&pool, accounts),
        pool.execute(query, (ledger_hash_str, epoch))
    )?;

    let json = Arc::new(json.to_vec());

    for chunk in json.chunks(chunk_size()) {
        let mut futures = Vec::new();

        for activity in chunk {
            let activity = activity.clone();
            let ledger_hash = Arc::clone(&ledger_hash);

            let balance = to_decimal(&activity["balance"]);
            let token = activity["token"].as_i64();
            let nonce = activity["nonce"].as_i64();
            let receipt_chain_hash = activity["receipt_chain_hash"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let voting_for = activity["voting_for"]
                .as_str()
                .unwrap_or_default()
                .to_string();

            let (query, params) = if let Some(timing) = activity["timing"].as_object() {
                let query = format!(
                    "with ledger := (
                        insert StakingLedger {{
                            epoch := assert_single((select StakingEpoch filter .epoch = {} and .hash = '{}')),
                            source := {},
                            balance := <decimal>$0,
                            target := {},
                            token := <int64>$1,
                            nonce := <optional int64>$2,
                            receipt_chain_hash := <str>$3,
                            voting_for := <str>$4
                        }} unless conflict
                    )
                    insert StakingTiming {{
                        ledger := ledger,
                        initial_minimum_balance := <decimal>$5,
                        cliff_time := <int64>$6,
                        cliff_amount := <decimal>$7,
                        vesting_period := <int64>$8,
                        vesting_increment := <decimal>$9
                    }} unless conflict;",
                    epoch,
                    ledger_hash,
                    account_link(&activity["pk"]),
                    account_link(&activity["delegate"])
                );

                let initial_minimum_balance = to_decimal(&timing["initial_minimum_balance"]);
                let cliff_time = timing["cliff_time"].as_i64();
                let cliff_amount = to_decimal(&timing["cliff_amount"]);
                let vesting_period = timing["vesting_period"].as_i64();
                let vesting_increment = to_decimal(&timing["vesting_increment"]);

                (
                    query,
                    (
                        balance,
                        token,
                        nonce,
                        receipt_chain_hash,
                        voting_for,
                        initial_minimum_balance,
                        cliff_time,
                        cliff_amount,
                        vesting_period,
                        vesting_increment,
                    ),
                )
            } else {
                let query = format!(
                    "insert StakingLedger {{
                        epoch := assert_single((select StakingEpoch filter .epoch = {} and .hash = '{}')),
                        source := {},
                        balance := <decimal>$0,
                        target := {},
                        token := <int64>$1,
                        nonce := <optional int64>$2,
                        receipt_chain_hash := <str>$3,
                        voting_for := <str>$4
                    }} unless conflict;",
                    epoch,
                    ledger_hash,
                    account_link(&activity["pk"]),
                    account_link(&activity["delegate"])
                );

                (
                    query,
                    (
                        balance,
                        token,
                        nonce,
                        receipt_chain_hash,
                        voting_for,
                        None::<BigDecimal>, // initial_minimum_balance
                        None::<i64>,        // cliff_time
                        None::<BigDecimal>, // cliff_amount
                        None::<i64>,        // vesting_period
                        None::<BigDecimal>, // vesting_increment
                    ),
                )
            };

            futures.push(pool.execute(query, params));
        }

        try_join_all(futures).await?;
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
