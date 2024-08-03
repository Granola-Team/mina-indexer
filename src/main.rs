use mysql::{prelude::*, *};
use regex::Regex;
use serde_json::Value;
use std::{fs, path::Path};
use uuid::Uuid;

fn main() -> Result<()> {
    let url = "mysql://username:password@localhost:3306/database_name";
    let pool = Pool::new(url)?;
    let mut conn = pool.get_conn()?;

    let entries = fs::read_dir(".").unwrap();
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
            process_file(&path, &mut conn)?;
        }
    }

    Ok(())
}

fn process_file(path: &Path, conn: &mut PooledConn) -> Result<()> {
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let block_hash = file_name
        .split('-')
        .nth(2)
        .unwrap()
        .split('.')
        .next()
        .unwrap();

    let contents = fs::read_to_string(path).unwrap();
    let json: Value = serde_json::from_str(&contents).unwrap();

    let height = json["protocol_state"]["body"]["consensus_state"]["blockchain_length"]
        .as_u64()
        .unwrap();
    println!("Processing block {} at height {}", block_hash, height);

    // Process accounts
    let accounts: Vec<String> = json
        .as_object()
        .unwrap()
        .values()
        .flat_map(|v| extract_accounts(v))
        .collect();

    for account in accounts {
        conn.exec_drop(
            "INSERT IGNORE INTO accounts (id) VALUES (:id)",
            params! {
                "id" => account,
            },
        )?;
    }

    // Process protocol_state
    conn.exec_drop(
        "INSERT INTO protocol_state (block_hash, previous_state_hash, genesis_state_hash, blockchain_length, min_window_density, total_currency, global_slot_since_genesis, has_ancestor_in_same_checkpoint_window, block_stake_winner, block_creator, coinbase_receiver, supercharge_coinbase) VALUES (:block_hash, :previous_state_hash, :genesis_state_hash, :blockchain_length, :min_window_density, :total_currency, :global_slot_since_genesis, :has_ancestor_in_same_checkpoint_window, :block_stake_winner, :block_creator, :coinbase_receiver, :supercharge_coinbase)",
        params! {
            "block_hash" => block_hash,
            "previous_state_hash" => json["protocol_state"]["previous_state_hash"].as_str().unwrap(),
            "genesis_state_hash" => json["protocol_state"]["body"]["genesis_state_hash"].as_str().unwrap(),
            "blockchain_length" => json["protocol_state"]["body"]["consensus_state"]["blockchain_length"].as_u64().unwrap(),
            "min_window_density" => json["protocol_state"]["body"]["consensus_state"]["min_window_density"].as_u64().unwrap(),
            "total_currency" => json["protocol_state"]["body"]["consensus_state"]["total_currency"].as_str().unwrap(),
            "global_slot_since_genesis" => json["protocol_state"]["body"]["consensus_state"]["global_slot_since_genesis"].as_u64().unwrap(),
            "has_ancestor_in_same_checkpoint_window" => json["protocol_state"]["body"]["consensus_state"]["has_ancestor_in_same_checkpoint_window"].as_bool().unwrap(),
            "block_stake_winner" => json["protocol_state"]["body"]["consensus_state"]["block_stake_winner"].as_str().unwrap(),
            "block_creator" => json["protocol_state"]["body"]["consensus_state"]["block_creator"].as_str().unwrap(),
            "coinbase_receiver" => json["protocol_state"]["body"]["consensus_state"]["coinbase_receiver"].as_str().unwrap(),
            "supercharge_coinbase" => json["protocol_state"]["body"]["consensus_state"]["supercharge_coinbase"].as_bool().unwrap(),
        },
    )?;

    // Process blockchain_state
    let blockchain_state_id = Uuid::new_v4().to_string();
    conn.exec_drop(
        "INSERT INTO blockchain_state (id, snarked_ledger_hash, genesis_ledger_hash, snarked_next_available_token, timestamp) VALUES (:id, :snarked_ledger_hash, :genesis_ledger_hash, :snarked_next_available_token, :timestamp)",
        params! {
            "id" => &blockchain_state_id,
            "snarked_ledger_hash" => json["protocol_state"]["body"]["blockchain_state"]["snarked_ledger_hash"].as_str().unwrap(),
            "genesis_ledger_hash" => json["protocol_state"]["body"]["blockchain_state"]["genesis_ledger_hash"].as_str().unwrap(),
            "snarked_next_available_token" => json["protocol_state"]["body"]["blockchain_state"]["snarked_next_available_token"].as_str().unwrap(),
            "timestamp" => json["protocol_state"]["body"]["blockchain_state"]["timestamp"].as_str().unwrap(),
        },
    )?;

    // Process consensus_state
    let consensus_state_id = Uuid::new_v4().to_string();
    conn.exec_drop(
        "INSERT INTO consensus_state (id, block_hash, epoch_count, curr_global_slot_slot_number, curr_global_slot_slots_per_epoch) VALUES (:id, :block_hash, :epoch_count, :curr_global_slot_slot_number, :curr_global_slot_slots_per_epoch)",
        params! {
            "id" => &consensus_state_id,
            "block_hash" => block_hash,
            "epoch_count" => json["protocol_state"]["body"]["consensus_state"]["epoch_count"].as_u64().unwrap(),
            "curr_global_slot_slot_number" => json["protocol_state"]["body"]["consensus_state"]["curr_global_slot"]["slot_number"].as_u64().unwrap(),
            "curr_global_slot_slots_per_epoch" => json["protocol_state"]["body"]["consensus_state"]["curr_global_slot"]["slots_per_epoch"].as_u64().unwrap(),
        },
    )?;

    // Process staged_ledger_hash
    conn.exec_drop(
        "INSERT INTO staged_ledger_hash (blockchain_state_id, non_snark_ledger_hash, non_snark_aux_hash, non_snark_pending_coinbase_aux, pending_coinbase_hash) VALUES (:blockchain_state_id, :non_snark_ledger_hash, :non_snark_aux_hash, :non_snark_pending_coinbase_aux, :pending_coinbase_hash)",
        params! {
            "blockchain_state_id" => &blockchain_state_id,
            "non_snark_ledger_hash" => json["protocol_state"]["body"]["blockchain_state"]["staged_ledger_hash"]["non_snark"]["ledger_hash"].as_str().unwrap(),
            "non_snark_aux_hash" => json["protocol_state"]["body"]["blockchain_state"]["staged_ledger_hash"]["non_snark"]["aux_hash"].as_str().unwrap(),
            "non_snark_pending_coinbase_aux" => json["protocol_state"]["body"]["blockchain_state"]["staged_ledger_hash"]["non_snark"]["pending_coinbase_aux"].as_str().unwrap(),
            "pending_coinbase_hash" => json["protocol_state"]["body"]["blockchain_state"]["staged_ledger_hash"]["pending_coinbase_hash"].as_str().unwrap(),
        },
    )?;

    // Process epoch_data
    for epoch_type in &["staking", "next"] {
        let epoch_data = &json["protocol_state"]["body"]["consensus_state"]
            [format!("{}_epoch_data", epoch_type)];
        conn.exec_drop(
            "INSERT INTO epoch_data (consensus_state_id, type, ledger_hash, total_currency, seed, start_checkpoint, lock_checkpoint, epoch_length) VALUES (:consensus_state_id, :type, :ledger_hash, :total_currency, :seed, :start_checkpoint, :lock_checkpoint, :epoch_length)",
            params! {
                "consensus_state_id" => &consensus_state_id,
                "type" => epoch_type,
                "ledger_hash" => epoch_data["ledger"]["hash"].as_str().unwrap(),
                "total_currency" => epoch_data["ledger"]["total_currency"].as_str().unwrap(),
                "seed" => epoch_data["seed"].as_str().unwrap(),
                "start_checkpoint" => epoch_data["start_checkpoint"].as_str().unwrap(),
                "lock_checkpoint" => epoch_data["lock_checkpoint"].as_str().unwrap(),
                "epoch_length" => epoch_data["epoch_length"].as_u64().unwrap(),
            },
        )?;
    }

    // Process sub_window_densities
    for density in json["protocol_state"]["body"]["consensus_state"]["sub_window_densities"]
        .as_array()
        .unwrap()
    {
        conn.exec_drop(
            "INSERT INTO sub_window_densities (consensus_state_id, density) VALUES (:consensus_state_id, :density)",
            params! {
                "consensus_state_id" => &consensus_state_id,
                "density" => density.as_u64().unwrap(),
            },
        )?;
    }

    // Process constants
    conn.exec_drop(
        "INSERT INTO constants (k, block_hash, slots_per_epoch, slots_per_sub_window, delta, genesis_state_timestamp) VALUES (:k, :block_hash, :slots_per_epoch, :slots_per_sub_window, :delta, :genesis_state_timestamp)",
        params! {
            "k" => json["protocol_state"]["body"]["constants"]["k"].as_u64().unwrap(),
            "block_hash" => block_hash,
            "slots_per_epoch" => json["protocol_state"]["body"]["constants"]["slots_per_epoch"].as_u64().unwrap(),
            "slots_per_sub_window" => json["protocol_state"]["body"]["constants"]["slots_per_sub_window"].as_u64().unwrap(),
            "delta" => json["protocol_state"]["body"]["constants"]["delta"].as_u64().unwrap(),
            "genesis_state_timestamp" => json["protocol_state"]["body"]["constants"]["genesis_state_timestamp"].as_str().unwrap(),
        },
    )?;

    // Process commands and command_status
    for command in json["staged_ledger_diff"]["diff"][0]["commands"]
        .as_array()
        .unwrap()
    {
        conn.exec_drop(
            "INSERT INTO commands (fee, fee_token, fee_payer_pk, nonce, valid_until, memo, source_pk, receiver_pk, token_id, amount, signer, signature) VALUES (:fee, :fee_token, :fee_payer_pk, :nonce, :valid_until, :memo, :source_pk, :receiver_pk, :token_id, :amount, :signer, :signature)",
            params! {
                "fee" => command["data"][1]["payload"]["common"]["fee"].as_str().unwrap(),
                "fee_token" => command["data"][1]["payload"]["common"]["fee_token"].as_str().unwrap(),
                "fee_payer_pk" => command["data"][1]["payload"]["common"]["fee_payer_pk"].as_str().unwrap(),
                "nonce" => command["data"][1]["payload"]["common"]["nonce"].as_str().unwrap(),
                "valid_until" => command["data"][1]["payload"]["common"]["valid_until"].as_str().unwrap(),
                "memo" => command["data"][1]["payload"]["common"]["memo"].as_str().unwrap(),
                "source_pk" => command["data"][1]["payload"]["body"][1]["source_pk"].as_str().unwrap(),
                "receiver_pk" => command["data"][1]["payload"]["body"][1]["receiver_pk"].as_str().unwrap(),
                "token_id" => command["data"][1]["payload"]["body"][1]["token_id"].as_str().unwrap(),
                "amount" => command["data"][1]["payload"]["body"][1]["amount"].as_str().unwrap(),
                "signer" => command["data"][1]["signer"].as_str().unwrap(),
                "signature" => command["data"][1]["signature"].as_str().unwrap(),
            },
        )?;

        conn.exec_drop(
            "INSERT INTO command_status (status, fee_payer_account_creation_fee_paid, receiver_account_creation_fee_paid, created_token, fee_payer_balance, source_balance, receiver_balance) VALUES (:status, :fee_payer_account_creation_fee_paid, :receiver_account_creation_fee_paid, :created_token, :fee_payer_balance, :source_balance, :receiver_balance)",
            params! {
                "status" => command["status"][0].as_str().unwrap(),
                "fee_payer_account_creation_fee_paid" => command["status"][1]["fee_payer_account_creation_fee_paid"].as_str().unwrap_or("NULL"),
                "receiver_account_creation_fee_paid" => command["status"][1]["receiver_account_creation_fee_paid"].as_str().unwrap_or("NULL"),
                "created_token" => command["status"][1]["created_token"].as_str().unwrap_or("NULL"),
                "fee_payer_balance" => command["status"][2]["fee_payer_balance"].as_str().unwrap(),
                "source_balance" => command["status"][2]["source_balance"].as_str().unwrap(),
                "receiver_balance" => command["status"][2]["receiver_balance"].as_str().unwrap(),
            },
        )?;
    }

    // Process coinbase and fee_transfer
    for internal_command in json["staged_ledger_diff"]["diff"][0]["internal_command_balances"]
        .as_array()
        .unwrap()
    {
        match internal_command[0].as_str().unwrap() {
            "Coinbase" => {
                conn.exec_drop(
                    "INSERT INTO coinbase (type, receiver_balance) VALUES (:type, :receiver_balance)",
                    params! {
                        "type" => "Coinbase",
                        "receiver_balance" => internal_command[1]["coinbase_receiver_balance"].as_str().unwrap(),
                    },
                )?;
            }
            "Fee_transfer" => {
                conn.exec_drop(
                    "INSERT INTO fee_transfer (receiver1_balance, receiver2_balance) VALUES (:receiver1_balance, :receiver2_balance)",
                    params! {
                        "receiver1_balance" => internal_command[1]["receiver1_balance"].as_str().unwrap(),
                        "receiver2_balance" => internal_command[1]["receiver2_balance"].as_str().unwrap_or("NULL"),
                    },
                )?;
            }
            _ => {}
        }
    }

    println!("Processed block {} at height {}", block_hash, height);
    Ok(())
}

fn extract_accounts(value: &Value) -> Vec<String> {
    let mut accounts = Vec::new();
    let re = Regex::new(r"^B62.{52}$").unwrap();

    if let Some(s) = value.as_str() {
        if re.is_match(s) {
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
