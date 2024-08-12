use bigdecimal::BigDecimal;
use edgedb_tokio::Client;
use regex::Regex;
use serde_json::Value;
use std::{fs, sync::LazyLock};

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

    let client = edgedb_tokio::create_client().await?;

    for path in paths {
        // println!("Attempting to read file {:?}", &path);
        match fs::read_to_string(&path) {
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

                match serde_json::from_str::<Value>(&contents) {
                    Ok(json) => {
                        process_file(block_hash, json, &client).await?;
                    }
                    Err(e) => {
                        // Handle the error gracefully
                        println!("Error - Failed to parse JSON for {:?}: {}", &path, e);
                    }
                }
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
    }

    Ok(())
}

async fn process_file(block_hash: &str, json: Value, client: &Client) -> anyhow::Result<()> {
    let body = &json["protocol_state"]["body"];
    let consensus_state = &body["consensus_state"];
    let blockchain_state = &body["blockchain_state"];

    let height = to_i64(&consensus_state["blockchain_length"]);

    println!("Processing block {} at height {}", block_hash, height);

    client
        .execute(
            "insert Block {
                hash := <str>$0,
                scheduled_time := <int64>$1
            };",
            &(block_hash, to_i64(&json["scheduled_time"])),
        )
        .await?;

    // Process accounts
    let accounts: Vec<String> = json
        .as_object()
        .unwrap()
        .values()
        .flat_map(|v| extract_accounts(v))
        .collect();

    for account in accounts {
        client
            .execute(
                "insert Account {public_key := <str>$0} unless conflict;",
                &(account,),
            )
            .await?;
    }

    // Process protocol_state
    client
        .execute(
            "
            insert ProtocolState {
                block := (select Block filter .hash = <str>$0),
                previous_state_hash := <str>$1,
                genesis_state_hash := <str>$2,
                height := <int64>$3,
                min_window_density := <int64>$4,
                total_currency := <int64>$5,
                global_slot_since_genesis := <int64>$6,
                has_ancestor_in_same_checkpoint_window := <bool>$7,
                block_stake_winner := (select Account filter .public_key = <str>$8),
                block_creator := (select Account filter .public_key = <str>$9),
                coinbase_receiver := (select Account filter .public_key = <str>$10),
                supercharge_coinbase := <bool>$11
            };",
            &(
                block_hash,
                json["protocol_state"]["previous_state_hash"].as_str(),
                body["genesis_state_hash"].as_str(),
                height,
                to_i64(&consensus_state["min_window_density"]),
                to_i64(&consensus_state["total_currency"]),
                to_i64(&consensus_state["global_slot_since_genesis"]),
                consensus_state["has_ancestor_in_same_checkpoint_window"].as_bool(),
                consensus_state["block_stake_winner"].as_str(),
                consensus_state["block_creator"].as_str(),
                consensus_state["coinbase_receiver"].as_str(),
                consensus_state["supercharge_coinbase"].as_bool(),
            ),
        )
        .await?;

    // Process blockchain_state
    client
        .execute(
            "
            with block := (select Block filter .hash = <str>$0)
            insert BlockchainState {
                protocol_state := assert_single((select ProtocolState filter .block = block)),
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

    // Process consensus_state
    client
        .execute(
            "
            with block := (select Block filter .hash = <str>$0)
            insert ConsensusState {
                protocol_state := assert_single((select ProtocolState filter .block = block)),
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

    // Process staged_ledger_hash
    let staged_ledger_hash = &blockchain_state["staged_ledger_hash"];
    let non_snark = &staged_ledger_hash["non_snark"];
    client
        .execute(
            "
            with
                block := (select Block filter .hash = <str>$0),
                protocol_state := assert_single((select ProtocolState filter .block = block))
            insert StagedLedgerHash {
                blockchain_state := assert_single((select BlockchainState filter .protocol_state = protocol_state)),
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
                staged_ledger_hash["pending_coinbase_hash"].as_str()
            ),
        )
        .await?;

    // Process epoch_data
    for epoch_type in ["staking", "next"] {
        let epoch_data = &consensus_state[format!("{}_epoch_data", epoch_type)];
        let ledger = &epoch_data["ledger"];
        client
            .execute(
                "
                with block := (select Block filter .hash = <str>$0)
                insert EpochData {
                    protocol_state := assert_single((select ProtocolState filter .block = block)),
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

    // Process commands and command_status
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
        client
            .execute(
                format!(
                    "
                    insert CommandStatus {{
                        command := (insert Command {{
                            block := (select Block filter .hash = '{}'),
                            fee := <decimal>$0,
                            fee_token := '{}',
                            fee_payer := (select Account filter .public_key = '{}'),
                            nonce := <int64>$1,
                            valid_until := <int64>$2,
                            memo := '{}',
                            source := (select Account filter .public_key = '{}'),
                            receiver := (select Account filter .public_key = '{}'),
                            token_id := <int64>$3,
                            amount := <decimal>$4,
                            signer := (select Account filter .public_key = '{}'),
                            signature := '{}'
                        }}),
                        status := <str>$5,
                        fee_payer_account_creation_fee_paid := <optional decimal>$6,
                        receiver_account_creation_fee_paid := <optional decimal>$7,
                        created_token := <optional str>$8,
                        fee_payer_balance := <decimal>$9,
                        source_balance := <decimal>$10,
                        receiver_balance := <decimal>$11,
                        }};",
                    block_hash,
                    common["fee_token"].as_str().unwrap(),
                    common["fee_payer_pk"].as_str().unwrap(),
                    common["memo"].as_str().unwrap(),
                    body1["source_pk"].as_str().unwrap(),
                    body1["receiver_pk"].as_str().unwrap(),
                    data1["signer"].as_str().unwrap(),
                    data1["signature"].as_str().unwrap()
                ),
                &(
                    to_decimal(&common["fee"]),
                    to_i64(&common["nonce"]),
                    to_i64(&common["valid_until"]),
                    to_i64(&body1["token_id"]),
                    to_decimal(&body1["amount"]),
                    status[0].as_str(),
                    to_decimal(&status_1["fee_payer_account_creation_fee_paid"]),
                    to_decimal(&status_1["receiver_account_creation_fee_paid"]),
                    status_1["created_token"].as_str(),
                    to_decimal(&status_2["fee_payer_balance"]),
                    to_decimal(&status_2["source_balance"]),
                    to_decimal(&status_2["receiver_balance"]),
                ),
            )
            .await?;
    }

    // Process coinbase and fee_transfer
    for internal_command in json["staged_ledger_diff"]["diff"][0]["internal_command_balances"]
        .as_array()
        .unwrap()
    {
        let internal_command_1 = &internal_command[1];
        match internal_command[0].as_str().unwrap() {
            "Coinbase" => {
                client
                    .execute(
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
                client
                    .execute(
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
    //     "Finished processing block {} at height {}...............................",
    //     block_hash, height
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
