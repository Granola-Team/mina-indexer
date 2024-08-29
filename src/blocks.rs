use edgedb_tokio::Client;
use regex::Regex;
use serde_json::Value;
use std::{
    collections::HashSet,
    sync::{Arc, LazyLock},
};
use tokio::sync::Semaphore;

use crate::{
    extract_hash_from_file_name, get_db, get_file_paths, insert_accounts, to_decimal, to_i64,
    to_json, to_titlecase,
};

const CONCURRENT_TASKS: usize = 5;

/// Ingest pre-computed block files (JSON) into the database
pub async fn run(blocks_dir: &str) -> anyhow::Result<()> {
    let semaphore = Arc::new(Semaphore::new(CONCURRENT_TASKS));
    let mut handles = vec![];

    let db = get_db(CONCURRENT_TASKS * 5).await?;

    for path in get_file_paths(blocks_dir)? {
        // clone the Arc to the semaphore for each task
        let sem = Arc::clone(&semaphore);
        let db = Arc::clone(&db);

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            match to_json(&path).await {
                Ok(json) => {
                    let block_hash = extract_hash_from_file_name(&path);

                    let a = insert(&db, json, block_hash).await;
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

/// Insert the `json` for a given `block_hash` into the `db`
async fn insert(db: &Arc<Client>, json: Value, block_hash: &str) -> anyhow::Result<()> {
    let protocol_state = &json["protocol_state"];
    let body = &protocol_state["body"];
    let consensus_state = &body["consensus_state"];
    let blockchain_state = &body["blockchain_state"];

    let height = to_i64(&consensus_state["blockchain_length"]).unwrap();

    println!("Processing block {} at height {}", block_hash, height);

    let accounts: HashSet<String> = json
        .as_object()
        .unwrap()
        .values()
        .flat_map(|v| extract_accounts(v))
        .collect();

    insert_accounts(db, accounts).await?;

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
                coinbase_target := (select Account filter .public_key = <str>$8),
                supercharge_coinbase := <bool>$9,
                has_ancestor_in_same_checkpoint_window := <bool>$10,
                min_window_density := <int64>$11
            }};",
            block_hash
        )
        .as_str(),
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
            format!(
                "
                insert {}EpochData {{
                    block := (select Block filter .hash = <str>$0),
                    ledger_hash := <str>$1,
                    total_currency := <int64>$2,
                    seed := <str>$3,
                    start_checkpoint := <str>$4,
                    lock_checkpoint := <str>$5,
                    epoch_length := <int64>$6
                }};",
                to_titlecase(epoch_type)
            )
            .as_str(),
            &(
                block_hash,
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
        let command = format!(
            "block := (select Block filter .hash = '{}'),
            status := '{}',
            source_balance := {}n,
            target_balance := {}n,
            fee := {}n,
            fee_payer := (select Account filter .public_key = '{}'),
            fee_payer_balance := {}n,
            fee_token := '{}',
            fee_payer_account_creation_fee_paid := {}n,
            target_account_creation_fee_paid := {}n,
            nonce := {},
            valid_until := {},
            memo := '{}',
            signer := (select Account filter .public_key = '{}'),
            signature := '{}',
            created_token := '{}'
            ",
            block_hash,
            status[0].as_str().unwrap(),
            to_decimal(&status_2["source_balance"]).unwrap(),
            // TODO: or default may be incorrect here since this is optional
            to_decimal(&status_2["receiver_balance"]).unwrap_or_default(),
            to_decimal(&common["fee"]).unwrap_or_default(),
            common["fee_payer_pk"].as_str().unwrap(),
            to_decimal(&status_2["fee_payer_balance"]).unwrap_or_default(),
            common["fee_token"].as_str().unwrap(),
            to_decimal(&status_1["fee_payer_account_creation_fee_paid"]).unwrap_or_default(),
            to_decimal(&status_1["receiver_account_creation_fee_paid"]).unwrap_or_default(),
            to_i64(&common["nonce"]).unwrap_or_default(),
            to_i64(&common["valid_until"]).unwrap_or_default(),
            // TODO: or default may be incorrect here since this is optional
            common["memo"].as_str().unwrap_or_default(),
            data1["signer"].as_str().unwrap(),
            data1["signature"].as_str().unwrap(),
            status_1["created_token"].as_str().unwrap_or_default(),
        );

        match payload["body"][0].as_str().unwrap() {
            "Stake_delegation" => {
                let delegation = &body1[1];
                db.execute(
                    format!(
                        "
                        insert StakingDelegation {{
                            {},
                            source := (select Account filter .public_key = <str>$0),
                            target := (select Account filter .public_key = <str>$1),
                        }};",
                        command
                    )
                    .as_str(),
                    &(
                        delegation["delegator"].as_str(),
                        delegation["new_delegate"].as_str(),
                    ),
                )
                .await?;
            }
            "Payment" => {
                db.execute(
                    format!(
                        "
                    insert Payment {{
                        {},
                        source := (select Account filter .public_key = <str>$0),
                        target := (select Account filter .public_key = <str>$1),
                        amount := <decimal>$2,
                        token_id := <int64>$3,
                    }};",
                        command
                    )
                    .as_str(),
                    &(
                        body1["source_pk"].as_str(),
                        body1["receiver_pk"].as_str(),
                        to_decimal(&body1["amount"]),
                        to_i64(&body1["token_id"]),
                    ),
                )
                .await?;
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
                            target_balance := <decimal>$1
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
                            target1_balance := <decimal>$1,
                            target2_balance := <optional decimal>$2
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

    Ok(())
}

const ACCOUNTS_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"B62.{52}$").expect("Failed to compile accounts regex"));

fn extract_accounts(value: &Value) -> HashSet<String> {
    let mut accounts = HashSet::new();

    match value {
        Value::String(s) if ACCOUNTS_REGEX.is_match(s) => {
            accounts.insert(s.to_string());
        }
        Value::Object(obj) => {
            for v in obj.values() {
                accounts.extend(extract_accounts(v));
            }
        }
        Value::Array(arr) => {
            for v in arr {
                accounts.extend(extract_accounts(v));
            }
        }
        _ => {}
    }

    accounts
}
