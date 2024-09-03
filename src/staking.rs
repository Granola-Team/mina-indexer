use std::{collections::HashSet, sync::Arc};

use edgedb_tokio::Client;
use sonic_rs::{JsonContainerTrait, JsonValueTrait, Value};
use tokio::sync::Semaphore;

use crate::{
    extract_digits_from_file_name, extract_hash_from_file_name, get_db, get_file_paths,
    insert_accounts, to_decimal, to_i64, to_json,
};

const CONCURRENT_TASKS: usize = 5;

/// Ingest staking ledger files (JSON) into the database
pub async fn run(staking_ledgers_dir: &str) -> anyhow::Result<()> {
    let semaphore = Arc::new(Semaphore::new(CONCURRENT_TASKS));
    let mut handles = vec![];

    let db = get_db(CONCURRENT_TASKS * CONCURRENT_TASKS).await?;

    for path in get_file_paths(staking_ledgers_dir)? {
        // clone the Arc to the semaphore for each task
        let sem = Arc::clone(&semaphore);
        let db = Arc::clone(&db);

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            match to_json(&path).await {
                Ok(json) => {
                    let ledger_hash = extract_hash_from_file_name(&path);
                    let epoch = extract_digits_from_file_name(&path);

                    let a = insert(&db, json, ledger_hash, epoch).await;
                    match a {
                        Ok(_) => (),
                        Err(e) => panic!("Ruhroh {:?}", e),
                    };
                }
                Err(e) => {
                    match e.kind() {
                        std::io::ErrorKind::InvalidData => {
                            println!("Error - Contains invalid UTF-8 data: {:?}", &path);
                        }
                        _ => {
                            // Handle other types of IO errors
                            println!("Error - Failed to read file {:?}: {}", &path, e);
                        }
                    }
                }
            }

            // permit is auto released when _permit goes out of scope
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await?;
    }
    Ok(())
}

async fn insert(
    db: &Arc<Client>,
    json: Value,
    ledger_hash: &str,
    epoch: i64,
) -> anyhow::Result<()> {
    let json = json.as_array().unwrap();
    let accounts = extract_accounts(json);
    insert_accounts(db, accounts).await?;

    db.execute(
        "insert StakingEpoch {
            hash := <str>$0,
            epoch := <int64>$1
        };",
        &(ledger_hash, epoch),
    )
    .await?;

    for activity in json {
        let source = activity["pk"].as_str();
        let balance = to_decimal(&activity["balance"]);
        let target = activity["delegate"].as_str();
        let token = to_i64(&activity["token"]);
        let nonce = to_i64(&activity["nonce"]);
        let receipt_chain_hash = activity["receipt_chain_hash"].as_str();
        let voting_for = activity["voting_for"].as_str();

        if let Some(timing) = activity["timing"].as_object() {
            let initial_minimum_balance = to_decimal(&timing["initial_minimum_balance"]);
            let cliff_time = to_i64(&timing["cliff_time"]);
            let cliff_amount = to_decimal(&timing["cliff_amount"]);
            let vesting_period = to_i64(&timing["vesting_period"]);
            let vesting_increment = to_decimal(&timing["vesting_increment"]);

            db.execute(
                format!("with ledger := (
                    insert StakingLedger {{
                        epoch := assert_single((select StakingEpoch filter .epoch = {} and .hash = '{}')),
                        source := (select Account filter .public_key = <str>$0),
                        balance := <decimal>$1,
                        target := (select Account filter .public_key = <str>$2),
                        token := <int64>$3,
                        nonce := <optional int64>$4,
                        receipt_chain_hash := <str>$5,
                        voting_for := <str>$6
                    }}
                )
                    insert StakingTiming {{
                        ledger := ledger,
                        initial_minimum_balance := <decimal>$7,
                        cliff_time := <int64>$8,
                        cliff_amount := <decimal>$9,
                        vesting_period := <int64>$10,
                        vesting_increment := <decimal>$11
                    }};", epoch, ledger_hash),
                &(
                    source,
                    balance,
                    target,
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
            .await?;
        } else {
            db.execute(
                "insert StakingLedger {
                    epoch := assert_single((select StakingEpoch filter .epoch = <int64>$0 and .hash = <str>$1)),
                    source := (select Account filter .public_key = <str>$2),
                    balance := <decimal>$3,
                    target := (select Account filter .public_key = <str>$4),
                    token := <int64>$5,
                    nonce := <optional int64>$6,
                    receipt_chain_hash := <str>$7,
                    voting_for := <str>$8
                }",
                &(
                    epoch,
                    ledger_hash,
                    source,
                    balance,
                    target,
                    token,
                    nonce,
                    receipt_chain_hash,
                    voting_for,
                ),
            )
            .await?;
        }
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
