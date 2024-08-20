use bigdecimal::BigDecimal;
use edgedb_tokio::Client;
use regex::Regex;
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, LazyLock},
};
use tokio::{
    fs::File,
    io::{self, AsyncReadExt, BufReader},
    sync::Semaphore,
};

const ACCOUNTS_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"B62.{52}$").expect("Failed to compile accounts regex"));

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut paths = fs::read_dir("/Users/jonathan/.mina-indexer/mina-indexer-dev/blocks-9999")?
        .filter_map(Result::ok) // Filter out any errors
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().map_or(false, |ext| ext == "json"))
        .collect::<Vec<_>>();

    // Sort by filename
    paths.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    let db_client = Arc::new(edgedb_tokio::create_client().await?);

    // 9 concurrent tasks
    let semaphore = Arc::new(Semaphore::new(9));
    let mut handles = vec![];

    for path in paths {
        // clone the Arc to the semaphore for each task
        let sem = Arc::clone(&semaphore);
        let db = Arc::clone(&db_client);

        let handle = tokio::spawn(async move {
            // acquire permit from semaphore
            let _permit = sem.acquire().await.unwrap();

            // println!("Attempting to read file {:?}", &path);
            match to_json(&path).await {
                Ok(contents) => {
                    // get block hash from file name
                    let file_name = path.file_name().unwrap().to_str().unwrap();
                    let block_hash = file_name
                        .split('-')
                        .nth(2)
                        .unwrap()
                        .split('.')
                        .next()
                        .unwrap();

                    let _ = process_file(block_hash, contents, &db).await;
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

            // the permit is automatically released when _permit goes out of
            // scope
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await?;
    }

    Ok(())
}

async fn process_file(block_hash: &str, json: Value, db: &Arc<Client>) -> anyhow::Result<()> {
    let protocol_state = &json["protocol_state"];
    let body = &protocol_state["body"];
    let consensus_state = &body["consensus_state"];
    let blockchain_state = &body["blockchain_state"];

    let height = to_i64(&consensus_state["blockchain_length"]);

    println!("Processing block {} at height {}", block_hash, height);

    // Process accounts
    let accounts: Vec<String> = json
        .as_object()
        .unwrap()
        .values()
        .flat_map(|v| extract_accounts(v))
        .collect();

    for account in accounts {
        db.execute(
            "insert Account {public_key := <str>$0} unless conflict;",
            &(account,),
        )
        .await?;
    }

    db.execute(
        format!(
            "insert Block {{
                hash := '{}',
                previous_hash := <str>$0,
                genesis_hash := <str>$1,
                height := <int64>$2,
                global_slot_since_genesis := <int64>$3,
                scheduled_time := <int64>$4,
                total_currency := <int64>$5,
                stake_winner := (select Account filter .public_key = <str>$6),
                creator := (select Account filter .public_key = <str>$7),
                coinbase_receiver := (select Account filter .public_key = <str>$8),
                supercharge_coinbase := <bool>$9,
                has_ancestor_in_same_checkpoint_window := <bool>$10,
                min_window_density := <int64>$11
            }};",
            block_hash
        ),
        &(
            protocol_state["previous_state_hash"].as_str(),
            body["genesis_state_hash"].as_str(),
            height,
            to_i64(&consensus_state["global_slot_since_genesis"]),
            to_i64(&json["scheduled_time"]),
            to_i64(&consensus_state["total_currency"]),
            consensus_state["block_stake_winner"].as_str(),
            consensus_state["block_creator"].as_str(),
            consensus_state["coinbase_receiver"].as_str(),
            consensus_state["supercharge_coinbase"].as_bool(),
            consensus_state["has_ancestor_in_same_checkpoint_window"].as_bool(),
            to_i64(&consensus_state["min_window_density"]),
        ),
    )
    .await?;

    // Process blockchain_state
    db.execute(
        "
            insert BlockchainState {
                block := (select Block filter .hash = <str>$0),
                snarked_ledger_hash := <str>$1,
                genesis_ledger_hash := <str>$2,
                snarked_next_available_token := <int64>$3,
                timestamp := <int64>$4
            };",
        &(
            block_hash,
            blockchain_state["snarked_ledger_hash"].as_str(),
            blockchain_state["genesis_ledger_hash"].as_str(),
            to_i64(&blockchain_state["snarked_next_available_token"]),
            to_i64(&blockchain_state["timestamp"]),
        ),
    )
    .await?;

    // Process staged_ledger_hash
    let staged_ledger_hash = &blockchain_state["staged_ledger_hash"];
    let non_snark = &staged_ledger_hash["non_snark"];
    db.execute(
        "
            with
                block := (select Block filter .hash = <str>$0)
            insert StagedLedgerHash {
                blockchain_state := assert_single((select BlockchainState filter .block = block)),
                non_snark_ledger_hash := <str>$1,
                non_snark_aux_hash := <str>$2,
                non_snark_pending_coinbase_aux := <str>$3,
                pending_coinbase_hash := <str>$4
            };",
        &(
            block_hash,
            non_snark["ledger_hash"].as_str(),
            non_snark["aux_hash"].as_str(),
            non_snark["pending_coinbase_aux"].as_str(),
            staged_ledger_hash["pending_coinbase_hash"].as_str(),
        ),
    )
    .await?;

    // Process consensus_state
    db.execute(
        "
            insert ConsensusState {
                block := (select Block filter .hash = <str>$0),
                epoch_count := <int64>$1,
                curr_global_slot_slot_number := <int64>$2,
                curr_global_slot_slots_per_epoch := <int64>$3
            };",
        &(
            block_hash,
            to_i64(&consensus_state["epoch_count"]),
            to_i64(&consensus_state["curr_global_slot"]["slot_number"]),
            to_i64(&consensus_state["curr_global_slot"]["slots_per_epoch"]),
        ),
    )
    .await?;

    // Process epoch_data
    for epoch_type in ["staking", "next"] {
        let epoch_data = &consensus_state[format!("{}_epoch_data", epoch_type)];
        let ledger = &epoch_data["ledger"];
        db.execute(
            "
                insert EpochData {
                    block := (select Block filter .hash = <str>$0),
                    type := <str>$1,
                    ledger_hash := <str>$2,
                    total_currency := <int64>$3,
                    seed := <str>$4,
                    start_checkpoint := <str>$5,
                    lock_checkpoint := <str>$6,
                    epoch_length := <int64>$7
                };",
            &(
                block_hash,
                epoch_type,
                ledger["hash"].as_str(),
                to_i64(&ledger["total_currency"]),
                epoch_data["seed"].as_str(),
                epoch_data["start_checkpoint"].as_str(),
                epoch_data["lock_checkpoint"].as_str(),
                to_i64(&epoch_data["epoch_length"]),
            ),
        )
        .await?;
    }

    for command in json["staged_ledger_diff"]["diff"][0]["commands"]
        .as_array()
        .unwrap()
    {
        let data1 = &command["data"][1];
        let payload = &data1["payload"];
        let common = &payload["common"];
        let body1 = &payload["body"][1];
        let status = &command["status"];
        let status_1 = &status[1];
        let status_2 = &status[2];

        // must use format!() since we have more than 12 query params
        // receiver_balance will be null when status == Failed
        let command: edgedb_protocol::value::Value = db
            .query_required_single(
                format!(
                    "
                    insert Command {{
                        block := (select Block filter .hash = '{}'),
                        status := <str>$0,
                        source_balance := <decimal>$1,
                        receiver_balance := <optional decimal>$2,
                        fee := <decimal>$3,
                        fee_payer := (select Account filter .public_key = '{}'),
                        fee_payer_balance := <decimal>$4,
                        fee_token := '{}',
                        fee_payer_account_creation_fee_paid := <optional decimal>$5,
                        receiver_account_creation_fee_paid := <optional decimal>$6,
                        nonce := <int64>$7,
                        valid_until := <int64>$8,
                        memo := <optional str>$9,
                        signer := (select Account filter .public_key = '{}'),
                        signature := '{}',
                        created_token := <optional str>$10,
                    }};",
                    block_hash,
                    common["fee_payer_pk"].as_str().unwrap(),
                    common["fee_token"].as_str().unwrap(),
                    data1["signer"].as_str().unwrap(),
                    data1["signature"].as_str().unwrap()
                ),
                &(
                    status[0].as_str(),
                    to_decimal(&status_2["source_balance"]),
                    to_decimal(&status_2["receiver_balance"]),
                    to_decimal(&common["fee"]),
                    to_decimal(&status_2["fee_payer_balance"]),
                    to_decimal(&status_1["fee_payer_account_creation_fee_paid"]),
                    to_decimal(&status_1["receiver_account_creation_fee_paid"]),
                    to_i64(&common["nonce"]),
                    to_i64(&common["valid_until"]),
                    common["memo"].as_str(),
                    status_1["created_token"].as_str(),
                ),
            )
            .await?;

        println!("command: {:?}", command);

        match payload["body"][0].as_str().unwrap() {
            "Stake_delegation" => {
                let delegation = &body1["Set_delegate"];
                db.execute(
                    "insert StakingDelegation {
                            command := <Enum>$0,
                            source := <str>$1,
                            receiver := str>$2
                        };",
                    &(
                        command,
                        delegation["delegator"].as_str(),
                        delegation["new_delegate"].as_str(),
                    ),
                )
                .await?;
            }
            "Payment" => {
                let a = db
                    .execute(
                        "insert Payment {
                            command := <Enum>$0,
                            source := (select Account filter .public_key = <str>$1),
                            receiver := (select Account filter .public_key = <str>$2),
                            amount := <decimal>$3,
                            token_id := <int64>$4,
                        };",
                        &(
                            command,
                            body1["source_pk"].as_str(),
                            body1["receiver_pk"].as_str(),
                            to_decimal(&body1["amount"]),
                            to_i64(&body1["token_id"]),
                        ),
                    )
                    .await;
                match a {
                    Ok(_) => println!("Payment was great"),
                    Err(e) => eprintln!("Uh oh: {:?}", e),
                }
            }
            _ => {
                println!("Unmatched {:?}", payload["body"][0].as_str().unwrap())
            }
        }
    }

    // Process coinbase and fee_transfer
    for internal_command in json["staged_ledger_diff"]["diff"][0]["internal_command_balances"]
        .as_array()
        .unwrap()
    {
        let internal_command_1 = &internal_command[1];
        match internal_command[0].as_str().unwrap() {
            "Coinbase" => {
                db.execute(
                    "insert Coinbase {
                            block := (select Block filter .hash = <str>$0),
                            receiver_balance := <decimal>$1
                        };",
                    &(
                        block_hash,
                        to_decimal(&internal_command_1["coinbase_receiver_balance"]),
                    ),
                )
                .await?;
            }
            "Fee_transfer" => {
                db.execute(
                    "insert FeeTransfer {
                            block := (select Block filter .hash = <str>$0),
                            receiver1_balance := <decimal>$1,
                            receiver2_balance := <optional decimal>$2
                        };",
                    &(
                        block_hash,
                        to_decimal(&internal_command_1["receiver1_balance"]),
                        to_decimal(&internal_command_1["receiver2_balance"]),
                    ),
                )
                .await?;
            }
            _ => {}
        }
    }

    // println!(
    //     "Finished processing block {} at height
    // {}...............................",     block_hash, height
    // );
    Ok(())
}

fn extract_accounts(value: &Value) -> Vec<String> {
    let mut accounts = Vec::new();

    if let Some(s) = value.as_str() {
        if ACCOUNTS_REGEX.is_match(s) {
            accounts.push(s.to_string());
        }
    } else if let Some(obj) = value.as_object() {
        for v in obj.values() {
            accounts.extend(extract_accounts(v));
        }
    } else if let Some(arr) = value.as_array() {
        for v in arr {
            accounts.extend(extract_accounts(v));
        }
    }

    accounts
}

/// These should really all be u64 but the conversion to EdgeDB requires i64
fn to_i64(value: &Value) -> i64 {
    value.as_str().and_then(|s| s.parse().ok()).unwrap()
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
