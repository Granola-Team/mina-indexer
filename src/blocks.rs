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

    let db = get_db(CONCURRENT_TASKS * CONCURRENT_TASKS).await?;

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

    let staged_ledger_hash = &blockchain_state["staged_ledger_hash"];
    let non_snark = &staged_ledger_hash["non_snark"];
    let global_slot = to_i64(&consensus_state["global_slot_since_genesis"]);

    db.execute(
        format!(
            "with
                block := (
                    insert Block {{
                        hash := '{}',
                        previous_hash := <str>$0,
                        genesis_hash := <str>$1,
                        height := <int64>$2,
                        global_slot_since_genesis := <int64>$3,
                        scheduled_time := <int64>$4,
                        total_currency := <int64>$5,
                        stake_winner := {},
                        creator := {},
                        coinbase_target := {},
                        supercharge_coinbase := <bool>$6,
                        has_ancestor_in_same_checkpoint_window := <bool>$7,
                        min_window_density := <int64>$8
                    }}
                ),
                blockchain_state := (
                    insert BlockchainState {{
                        block := block,
                        snarked_ledger_hash := '{}',
                        genesis_ledger_hash := '{}',
                        snarked_next_available_token := {},
                        timestamp := {}
                    }}
                ),
                consensus_state := (
                    insert ConsensusState {{
                        block := block,
                        epoch_count := {},
                        curr_global_slot_slot_number := {}
                    }}
                )
            insert StagedLedgerHash {{
                blockchain_state := blockchain_state,
                non_snark_ledger_hash := '{}',
                non_snark_aux_hash := '{}',
                non_snark_pending_coinbase_aux := '{}',
                pending_coinbase_hash := '{}'
            }}
            ;",
            block_hash,
            account_link(&consensus_state["block_stake_winner"]),
            account_link(&consensus_state["block_creator"]),
            account_link(&consensus_state["coinbase_receiver"]),
            blockchain_state["snarked_ledger_hash"]
                .as_str()
                .expect("snarked_ledger_hash is missing"),
            blockchain_state["genesis_ledger_hash"]
                .as_str()
                .expect("genesis_ledger_hash is missing"),
            to_i64(&blockchain_state["snarked_next_available_token"])
                .expect("snarked_next_available_token is missing"),
            to_i64(&blockchain_state["timestamp"]).expect("timestamp is missing"),
            to_i64(&consensus_state["epoch_count"]).expect("epoch_count is missing"),
            to_i64(&consensus_state["curr_global_slot"]["slot_number"])
                .expect("slot_number is missing"),
            to_i64(&consensus_state["curr_global_slot"]["slots_per_epoch"])
                .expect("slots_per_epoch is missing"),
            non_snark["ledger_hash"]
                .as_str()
                .expect("ledger_hash is missing"),
            non_snark["aux_hash"].as_str().expect("aux_hash is missing"),
            non_snark["pending_coinbase_aux"]
                .as_str()
                .expect("pending_coinbase_aux is missing"),
            staged_ledger_hash["pending_coinbase_hash"]
                .as_str()
                .expect("pending_coinbase_hash is missing")
        )
        .as_str(),
        &(
            protocol_state["previous_state_hash"].as_str(),
            body["genesis_state_hash"].as_str(),
            height,
            to_i64(&consensus_state["global_slot_since_genesis"]),
            to_i64(&json["scheduled_time"]),
            to_i64(&consensus_state["total_currency"]),
            consensus_state["supercharge_coinbase"].as_bool(),
            consensus_state["has_ancestor_in_same_checkpoint_window"].as_bool(),
            to_i64(&consensus_state["min_window_density"]),
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
                    block := {},
                    ledger_hash := <str>$0,
                    total_currency := <int64>$1,
                    seed := <str>$2,
                    start_checkpoint := <str>$3,
                    lock_checkpoint := <str>$4,
                    epoch_length := <int64>$5
                }};",
                to_titlecase(epoch_type),
                block_link(block_hash)
            )
            .as_str(),
            &(
                ledger["hash"].as_str().expect("ledger_hash is missing"),
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
            "block := {},
            status := '{}',
            source_balance := {}n,
            target_balance := {}n,
            fee := {}n,
            fee_payer := {},
            fee_payer_balance := {}n,
            fee_token := '{}',
            fee_payer_account_creation_fee_paid := {}n,
            target_account_creation_fee_paid := {}n,
            nonce := {},
            valid_until := {},
            memo := '{}',
            signer := {},
            signature := '{}',
            created_token := '{}'
            ",
            block_link(block_hash),
            status[0].as_str().unwrap(),
            to_decimal(&status_2["source_balance"]).unwrap(),
            // TODO: or default may be incorrect here since this is optional
            to_decimal(&status_2["receiver_balance"]).unwrap_or_default(),
            to_decimal(&common["fee"]).unwrap_or_default(),
            account_link(&common["fee_payer_pk"]),
            to_decimal(&status_2["fee_payer_balance"]).unwrap_or_default(),
            common["fee_token"].as_str().unwrap(),
            to_decimal(&status_1["fee_payer_account_creation_fee_paid"]).unwrap_or_default(),
            to_decimal(&status_1["receiver_account_creation_fee_paid"]).unwrap_or_default(),
            to_i64(&common["nonce"]).unwrap_or_default(),
            to_i64(&common["valid_until"]).unwrap_or_default(),
            // TODO: or default may be incorrect here since this is optional
            common["memo"].as_str().unwrap_or_default(),
            account_link(&data1["signer"]),
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
                            source := {},
                            target := {},
                        }};",
                        command,
                        account_link(&delegation["delegator"]),
                        account_link(&delegation["new_delegate"])
                    )
                    .as_str(),
                    &(),
                )
                .await?;
            }
            "Payment" => {
                db.execute(
                    format!(
                        "
                    insert Payment {{
                        {},
                        source := {},
                        target := {},
                        amount := <decimal>$0,
                        token_id := <int64>$1,
                    }};",
                        command,
                        account_link(&body1["source_pk"]),
                        account_link(&body1["receiver_pk"]),
                    )
                    .as_str(),
                    &(to_decimal(&body1["amount"]), to_i64(&body1["token_id"])),
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
                    format!(
                        "insert Coinbase {{
                            block := {},
                            target_balance := <decimal>$0
                        }};",
                        block_link(block_hash)
                    ),
                    &(to_decimal(&internal_command_1["coinbase_receiver_balance"]),),
                )
                .await?;
            }
            "Fee_transfer" => {
                db.execute(
                    format!(
                        "insert FeeTransfer {{
                            block := {},
                            target1_balance := <decimal>$0,
                            target2_balance := <optional decimal>$1
                        }};",
                        block_link(block_hash)
                    ),
                    &(
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

fn account_link(public_key: &Value) -> String {
    return format!(
        "(select Account filter .public_key = '{}')",
        public_key.as_str().unwrap()
    );
}

fn block_link(hash: &str) -> String {
    return format!("(select Block filter .hash = '{}')", hash);
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
