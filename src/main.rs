use mysql::{prelude::*, *};
use regex::Regex;
use serde_json::Value;
use std::{fs, sync::LazyLock};
use uuid::Uuid;

const ACCOUNTS_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"B62.{52}$").expect("Failed to compile accounts regex"));

fn main() -> Result<()> {
    let mut conn = Pool::new("mysql://root:password@127.0.0.1:3306/blocks-9999")?.get_conn()?;

    let mut paths = fs::read_dir("/Users/jonathan/.mina-indexer/mina-indexer-dev/blocks-9999")?
        .filter_map(Result::ok) // Filter out any errors
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().map_or(false, |ext| ext == "json"))
        .collect::<Vec<_>>();

    // Sort by filename
    paths.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

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
                        process_file(block_hash, json, &mut conn)?;
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

fn process_file(block_hash: &str, json: Value, conn: &mut PooledConn) -> Result<()> {
    let body = &json["protocol_state"]["body"];
    let consensus_state = &body["consensus_state"];
    let blockchain_state = &body["blockchain_state"];

    let height = to_u64(&consensus_state["blockchain_length"]);

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
            "previous_state_hash" => json["protocol_state"]["previous_state_hash"].as_str(),
            "genesis_state_hash" => body["genesis_state_hash"].as_str(),
            "blockchain_length" => height,
            "min_window_density" => to_u64(&consensus_state["min_window_density"]),
            "total_currency" => consensus_state["total_currency"].as_str(),
            "global_slot_since_genesis" => to_u64(&consensus_state["global_slot_since_genesis"]),
            "has_ancestor_in_same_checkpoint_window" => consensus_state["has_ancestor_in_same_checkpoint_window"].as_bool(),
            "block_stake_winner" => consensus_state["block_stake_winner"].as_str(),
            "block_creator" => consensus_state["block_creator"].as_str(),
            "coinbase_receiver" => consensus_state["coinbase_receiver"].as_str(),
            "supercharge_coinbase" => consensus_state["supercharge_coinbase"].as_bool(),
        },
    )?;

    // println!("inserted protocol state...............................");

    // Process blockchain_state
    let blockchain_state_id = Uuid::new_v4().to_string();
    conn.exec_drop(
        "INSERT INTO blockchain_state (id, block_hash, snarked_ledger_hash, genesis_ledger_hash, snarked_next_available_token, timestamp) VALUES (:id, :block_hash, :snarked_ledger_hash, :genesis_ledger_hash, :snarked_next_available_token, :timestamp)",
        params! {
            "id" => &blockchain_state_id,
            "block_hash" => block_hash,
            "snarked_ledger_hash" => blockchain_state["snarked_ledger_hash"].as_str(),
            "genesis_ledger_hash" => blockchain_state["genesis_ledger_hash"].as_str(),
            "snarked_next_available_token" => blockchain_state["snarked_next_available_token"].as_str(),
            "timestamp" => blockchain_state["timestamp"].as_str(),
        },
    )?;

    // println!("inserted blockchain_state...............................");

    // Process consensus_state
    let consensus_state_id = Uuid::new_v4().to_string();
    conn.exec_drop(
        "INSERT INTO consensus_state (id, block_hash, epoch_count, curr_global_slot_slot_number, curr_global_slot_slots_per_epoch) VALUES (:id, :block_hash, :epoch_count, :curr_global_slot_slot_number, :curr_global_slot_slots_per_epoch)",
        params! {
            "id" => &consensus_state_id,
            "block_hash" => block_hash,
            "epoch_count" => to_u64(&consensus_state["epoch_count"]),
            "curr_global_slot_slot_number" => to_u64(&consensus_state["curr_global_slot"]["slot_number"]),
            "curr_global_slot_slots_per_epoch" => to_u64(&consensus_state["curr_global_slot"]["slots_per_epoch"]),
        },
    )?;

    // println!("inserted consensus_state_id...............................");

    // Process staged_ledger_hash
    let staged_ledger_hash = &blockchain_state["staged_ledger_hash"];
    let non_snark = &staged_ledger_hash["non_snark"];
    conn.exec_drop(
        "INSERT INTO staged_ledger_hash (blockchain_state_id, non_snark_ledger_hash, non_snark_aux_hash, non_snark_pending_coinbase_aux, pending_coinbase_hash) VALUES (:blockchain_state_id, :non_snark_ledger_hash, :non_snark_aux_hash, :non_snark_pending_coinbase_aux, :pending_coinbase_hash)",
        params! {
            "blockchain_state_id" => &blockchain_state_id,
            "non_snark_ledger_hash" => non_snark["ledger_hash"].as_str(),
            "non_snark_aux_hash" => non_snark["aux_hash"].as_str(),
            "non_snark_pending_coinbase_aux" => non_snark["pending_coinbase_aux"].as_str(),
            "pending_coinbase_hash" => staged_ledger_hash["pending_coinbase_hash"].as_str(),
        },
    )?;

    // println!("inserted staged_ledger_hash...............................");

    // Process epoch_data
    for epoch_type in &["staking", "next"] {
        let epoch_data = &consensus_state[format!("{}_epoch_data", epoch_type)];
        let ledger = &epoch_data["ledger"];
        conn.exec_drop(
            "INSERT INTO epoch_data (consensus_state_id, type, ledger_hash, total_currency, seed, start_checkpoint, lock_checkpoint, epoch_length) VALUES (:consensus_state_id, :type, :ledger_hash, :total_currency, :seed, :start_checkpoint, :lock_checkpoint, :epoch_length)",
            params! {
                "consensus_state_id" => &consensus_state_id,
                "type" => epoch_type,
                "ledger_hash" => ledger["hash"].as_str(),
                "total_currency" => ledger["total_currency"].as_str(),
                "seed" => epoch_data["seed"].as_str(),
                "start_checkpoint" => epoch_data["start_checkpoint"].as_str(),
                "lock_checkpoint" => epoch_data["lock_checkpoint"].as_str(),
                "epoch_length" => to_u64(&epoch_data["epoch_length"]),
            },
        )?;
    }

    // println!("inserted epoch_data...............................");

    // Process commands and command_status
    for command in json["staged_ledger_diff"]["diff"][0]["commands"]
        .as_array()
        .unwrap()
    {
        let data1 = &command["data"][1];
        let payload = &data1["payload"];
        let common = &payload["common"];
        let body1 = &payload["body"][1];
        conn.exec_drop(
            "INSERT INTO commands (fee, fee_token, fee_payer_pk, nonce, valid_until, memo, source_pk, receiver_pk, token_id, amount, signer, signature) VALUES (:fee, :fee_token, :fee_payer_pk, :nonce, :valid_until, :memo, :source_pk, :receiver_pk, :token_id, :amount, :signer, :signature)",
            params! {
                "fee" => common["fee"].as_str(),
                "fee_token" => common["fee_token"].as_str(),
                "fee_payer_pk" => common["fee_payer_pk"].as_str(),
                "nonce" => common["nonce"].as_str(),
                "valid_until" => common["valid_until"].as_str(),
                "memo" => common["memo"].as_str(),
                "source_pk" => body1["source_pk"].as_str(),
                "receiver_pk" => body1["receiver_pk"].as_str(),
                "token_id" => body1["token_id"].as_str(),
                "amount" => body1["amount"].as_str(),
                "signer" => data1["signer"].as_str(),
                "signature" => data1["signature"].as_str(),
            },
        )?;

        // println!("inserted commands...............................");

        let status = &command["status"];
        let status_1 = &status[1];
        let status_2 = &status[2];
        conn.exec_drop(
            "INSERT INTO command_status (status, fee_payer_account_creation_fee_paid, receiver_account_creation_fee_paid, created_token, fee_payer_balance, source_balance, receiver_balance) VALUES (:status, :fee_payer_account_creation_fee_paid, :receiver_account_creation_fee_paid, :created_token, :fee_payer_balance, :source_balance, :receiver_balance)",
            params! {
                "status" => status[0].as_str(),
                "fee_payer_account_creation_fee_paid" => status_1["fee_payer_account_creation_fee_paid"].as_str(),
                "receiver_account_creation_fee_paid" => status_1["receiver_account_creation_fee_paid"].as_str(),
                "created_token" => status_1["created_token"].as_str(),
                "fee_payer_balance" => status_2["fee_payer_balance"].as_str(),
                "source_balance" => status_2["source_balance"].as_str(),
                "receiver_balance" => status_2["receiver_balance"].as_str(),
            },
        )?;

        // println!("inserted command_status...............................");
    }

    // Process coinbase and fee_transfer
    for internal_command in json["staged_ledger_diff"]["diff"][0]["internal_command_balances"]
        .as_array()
        .unwrap()
    {
        let internal_command_1 = &internal_command[1];
        match internal_command[0].as_str().unwrap() {
            "Coinbase" => {
                conn.exec_drop(
                    "INSERT INTO coinbase (type, receiver_balance) VALUES (:type, :receiver_balance)",
                    params! {
                        "type" => "Coinbase",
                        "receiver_balance" => internal_command_1["coinbase_receiver_balance"].as_str(),
                    },
                )?;
                // println!("inserted coinbase...............................");
            }
            "Fee_transfer" => {
                conn.exec_drop(
                    "INSERT INTO fee_transfer (receiver1_balance, receiver2_balance) VALUES (:receiver1_balance, :receiver2_balance)",
                    params! {
                        "receiver1_balance" => internal_command_1["receiver1_balance"].as_str(),
                        "receiver2_balance" => internal_command_1["receiver2_balance"].as_str(),
                    },
                )?;
                // println!("inserted fee_transfer...............................");
            }
            _ => {}
        }
    }

    println!(
        "Finished processing block {} at height {}...............................",
        block_hash, height
    );
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

fn to_u64(value: &Value) -> u64 {
    value.as_str().and_then(|s| s.parse().ok()).unwrap()
}
