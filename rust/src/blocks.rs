use crate::{files::process_files, get_db_connection, insert_accounts, to_decimal, to_i64};
use anyhow::Result;
use duckdb::params;
use regex::Regex;
use sonic_rs::{Array, JsonContainerTrait, JsonType, JsonValueTrait, Value};
use std::{collections::HashSet, sync::LazyLock};

pub async fn run(blocks_dir: String) -> Result<()> {
    process_files(&blocks_dir, |json, hash, number| async move { process_block(json, hash, number).await }).await
}

async fn process_block(json: Value, block_hash: String, height: i64) -> Result<(), duckdb::Error> {
    let accounts = extract_accounts(&json);
    insert_accounts(accounts)?;

    let protocol_state = &json["protocol_state"];
    let body = &protocol_state["body"];
    let blockchain_state = &body["blockchain_state"];
    let consensus_state = &body["consensus_state"];
    let scheduled_time = to_i64(&json["scheduled_time"]).expect("scheduled_time");
    let staged_ledger_hash = &json["staged_ledger_hash"];
    let non_snark = &json["non_snark"];

    process_block_data(
        &block_hash,
        protocol_state,
        blockchain_state,
        scheduled_time,
        consensus_state,
        staged_ledger_hash,
        non_snark,
        height,
    )?;

    let (_, _) = tokio::try_join!(
        process_epoch_data(&block_hash, consensus_state),
        process_commands(&block_hash, json["staged_ledger_diff"]["diff"].as_array())
    )?;

    Ok(())
}

fn process_block_data(
    block_hash: &str,
    protocol_state: &Value,
    blockchain_state: &Value,
    scheduled_time: i64,
    consensus_state: &Value,
    staged_ledger_hash: &Value,
    non_snark: &Value,
    height: i64,
) -> Result<(), duckdb::Error> {
    let db = get_db_connection()?;

    db.execute(
        "INSERT INTO blocks (
            hash,
            previous_hash,
            genesis_hash,
            blockchain_length,
            epoch,
            global_slot_since_genesis,
            scheduled_time,
            total_currency,
            stake_winner,
            creator,
            coinbase_target,
            supercharge_coinbase,
            has_ancestor_in_same_checkpoint_window,
            min_window_density,
            last_vrf_output
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            block_hash,
            protocol_state["previous_state_hash"].as_str().expect("previous_hash"),
            blockchain_state["genesis_state_hash"].as_str().expect("genesis_hash"),
            height,
            to_i64(&consensus_state["epoch_count"]).expect("epoch"),
            to_i64(&consensus_state["global_slot_since_genesis"]).expect("global_slot"),
            scheduled_time,
            to_i64(&consensus_state["total_currency"]).expect("total_currency"),
            consensus_state["stake_winner"].as_str().expect("stake_winner"),
            consensus_state["block_creator"].as_str().expect("block_creator"),
            consensus_state["coinbase_receiver"].as_str().expect("coinbase_receiver"),
            consensus_state["supercharge_coinbase"].as_bool().unwrap_or(false),
            consensus_state["has_ancestor_in_same_checkpoint_window"].as_bool().unwrap_or(false),
            to_i64(&consensus_state["min_window_density"]).expect("min_window_density"),
            consensus_state["last_vrf_output"].as_str().expect("last_vrf_output")
        ],
    )?;

    db.execute(
        "INSERT INTO blockchain_states (
            block_hash,
            snarked_ledger_hash,
            genesis_ledger_hash,
            snarked_next_available_token,
            timestamp
        ) VALUES (?, ?, ?, ?, ?)",
        params![
            block_hash,
            blockchain_state["snarked_ledger_hash"].as_str().expect("snarked_ledger_hash"),
            blockchain_state["genesis_ledger_hash"].as_str().expect("genesis_ledger_hash"),
            to_i64(&blockchain_state["snarked_next_available_token"]).expect("snarked_next_available_token"),
            to_i64(&blockchain_state["timestamp"]).expect("timestamp")
        ],
    )?;

    db.execute(
        "INSERT INTO staged_ledger_hashes (
            blockchain_state_hash,
            non_snark_ledger_hash,
            non_snark_aux_hash,
            non_snark_pending_coinbase_aux,
            pending_coinbase_hash
        ) VALUES (?, ?, ?, ?, ?)",
        params![
            block_hash,
            non_snark["ledger_hash"].as_str().expect("ledger_hash"),
            non_snark["aux_hash"].as_str().expect("aux_hash"),
            non_snark["pending_coinbase_aux"].as_str().expect("pending_coinbase_aux"),
            staged_ledger_hash["pending_coinbase_hash"].as_str().expect("pending_coinbase_hash")
        ],
    )?;

    Ok(())
}

async fn process_epoch_data(block_hash: &str, consensus_state: &Value) -> Result<(), duckdb::Error> {
    let db = get_db_connection()?;

    for epoch_type in ["staking", "next"] {
        let epoch_data = &consensus_state[format!("{}_epoch_data", epoch_type).as_str()];
        let ledger = &epoch_data["ledger"];

        db.execute(
            "INSERT INTO epoch_data (
                block_hash,
                ledger_hash,
                total_currency,
                seed,
                start_checkpoint,
                lock_checkpoint,
                epoch_length,
                type
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                block_hash,
                ledger["hash"].as_str().expect("ledger hash"),
                to_i64(&ledger["total_currency"]).expect("total_currency"),
                epoch_data["seed"].as_str().expect("seed"),
                epoch_data["start_checkpoint"].as_str().expect("start_checkpoint"),
                epoch_data["lock_checkpoint"].as_str().expect("lock_checkpoint"),
                to_i64(&epoch_data["epoch_length"]).expect("epoch_length"),
                epoch_type
            ],
        )?;
    }

    Ok(())
}

async fn process_commands(block_hash: &str, diffs: Option<&Array>) -> Result<(), duckdb::Error> {
    if let Some(diffs) = diffs {
        for diff in diffs {
            snark_jobs(block_hash, diff)?;
            user_commands(block_hash, diff)?;
            internal_commands(block_hash, diff)?;
        }
    }
    Ok(())
}

fn snark_jobs(block_hash: &str, diff: &Value) -> Result<(), duckdb::Error> {
    let db = get_db_connection()?;
    if let Some(completed_works) = diff["completed_works"].as_array() {
        for job in completed_works {
            db.execute(
                "INSERT INTO snark_jobs (block_hash, prover, fee) VALUES (?, ?, ?)",
                params![
                    block_hash,
                    job["prover"].as_str().expect("SNARK job prover is missing"),
                    to_decimal(&job["fee"]).expect("SNARK job fee is missing").to_string()
                ],
            )?;
        }
    }
    Ok(())
}

fn user_commands(block_hash: &str, diff: &Value) -> Result<(), duckdb::Error> {
    let db = get_db_connection()?;
    if let Some(user_commands) = diff["user_commands"].as_array() {
        for command in user_commands {
            let data1 = &command["data"][1];
            let payload = &data1["payload"];
            let common = &payload["common"];
            let body1 = &payload["body"][1];
            let status = &command["status"];
            let status_1 = &status[1];
            let status_2 = &status[2];

            let cmd_type = payload["body"][0].as_str().expect("command type");

            db.execute(
                "INSERT INTO user_commands (
                    block_hash,
                    type,
                    source,
                    target,
                    amount,
                    fee,
                    token_id,
                    status,
                    source_balance,
                    target_balance,
                    fee_payer,
                    fee_payer_balance,
                    fee_token,
                    fee_payer_account_creation_fee_paid,
                    target_account_creation_fee_paid,
                    nonce,
                    valid_until,
                    memo,
                    signer,
                    signature,
                    created_token
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    block_hash,
                    match cmd_type {
                        "Stake_delegation" => "staking_delegation",
                        "Payment" => "payment",
                        _ => continue,
                    },
                    match cmd_type {
                        "Stake_delegation" => body1["delegator"].as_str().expect("delegator"),
                        "Payment" => body1["source_pk"].as_str().expect("source_pk"),
                        _ => continue,
                    },
                    match cmd_type {
                        "Stake_delegation" => body1["new_delegate"].as_str().expect("new_delegate"),
                        "Payment" => body1["receiver_pk"].as_str().expect("receiver_pk"),
                        _ => continue,
                    },
                    to_decimal(&body1["amount"]).expect("amount").to_string(),
                    to_decimal(&common["fee"]).expect("fee").to_string(),
                    to_i64(&body1["token_id"]).expect("token_id").to_string(),
                    status[0].as_str().expect("status"),
                    to_decimal(&status_2["source_balance"]).expect("source_balance").to_string(),
                    to_decimal(&status_2["receiver_balance"]).expect("receiver_balance").to_string(),
                    common["fee_payer_pk"].as_str().expect("fee_payer"),
                    to_decimal(&status_2["fee_payer_balance"]).expect("fee_payer_balance").to_string(),
                    common["fee_token"].as_str().expect("fee_token"),
                    to_decimal(&status_1["fee_payer_account_creation_fee_paid"])
                        .expect("fee_payer_account_creation_fee_paid")
                        .to_string(),
                    to_decimal(&status_1["receiver_account_creation_fee_paid"])
                        .expect("receiver_account_creation_fee_paid")
                        .to_string(),
                    to_i64(&common["nonce"]).expect("nonce").to_string(),
                    to_i64(&common["valid_until"]).expect("valid_until").to_string(),
                    common["memo"].as_str().expect("memo"),
                    data1["signer"].as_str().expect("signer"),
                    data1["signature"].as_str().expect("signature"),
                    status_1["created_token"].as_str().unwrap_or_default()
                ],
            )?;
        }
    }
    Ok(())
}

fn internal_commands(block_hash: &str, diff: &Value) -> Result<(), duckdb::Error> {
    let db = get_db_connection()?;
    if let Some(internal_commands) = diff["internal_command_balances"].as_array() {
        for internal_command in internal_commands {
            let internal_command_1 = &internal_command[1];
            match internal_command[0].as_str().expect("internal command type") {
                "Coinbase" => {
                    db.execute(
                        "INSERT INTO internal_commands (
                            block_hash,
                            type,
                            target1_balance
                        ) VALUES (?, 'coinbase', ?)",
                        params![
                            block_hash,
                            to_decimal(&internal_command_1["coinbase_receiver_balance"])
                                .expect("coinbase_receiver_balance is missing")
                                .to_string()
                        ],
                    )?;
                }
                "Fee_transfer" => {
                    db.execute(
                        "INSERT INTO internal_commands (
                            block_hash,
                            type,
                            target1_balance,
                            target2_balance
                        ) VALUES (?, 'fee_transfer', ?, ?)",
                        params![
                            block_hash,
                            to_decimal(&internal_command_1["receiver1_balance"])
                                .expect("receiver1_balance is missing")
                                .to_string(),
                            to_decimal(&internal_command_1["receiver2_balance"])
                                .expect("receiver2_balance is missing")
                                .to_string()
                        ],
                    )?;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

static ACCOUNTS_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"B62.{52}$").expect("Failed to compile accounts regex"));

fn extract_accounts(value: &Value) -> HashSet<String> {
    let mut accounts = HashSet::new();

    match value.get_type() {
        JsonType::String => {
            if let Some(s) = value.as_str() {
                if ACCOUNTS_REGEX.is_match(s) {
                    accounts.insert(s.to_owned());
                }
            }
        }
        JsonType::Object => {
            if let Some(obj) = value.as_object() {
                for (_, v) in obj.iter() {
                    accounts.extend(extract_accounts(v));
                }
            }
        }
        JsonType::Array => {
            if let Some(arr) = value.as_array() {
                for v in arr {
                    accounts.extend(extract_accounts(v));
                }
            }
        }
        _ => {}
    }

    accounts
}
