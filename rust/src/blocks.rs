use regex::Regex;
use sonic_rs::{Array, JsonContainerTrait, JsonType, JsonValueTrait, Value};
use std::{
    collections::HashSet,
    sync::{Arc, LazyLock},
};

use crate::{
    account_link, db::DbPool, insert_accounts, process_files, to_decimal, to_i64, to_titlecase,
};

pub async fn run(blocks_dir: &str) -> anyhow::Result<()> {
    let pool = Arc::new(DbPool::new().await?);
    process_files(blocks_dir, pool, process_block).await
}

async fn process_block(
    pool: Arc<DbPool>,
    json: Value,
    block_hash: String,
    height: i64,
) -> Result<(), edgedb_tokio::Error> {
    let accounts = extract_accounts(&json);

    // First insert all accounts and wait for completion since blocks link to accounts
    insert_accounts(&pool, accounts).await?;

    let protocol_state = &json["protocol_state"];
    let body = &protocol_state["body"];
    let blockchain_state = &body["blockchain_state"];
    let consensus_state = &body["consensus_state"];
    let scheduled_time = to_i64(&json["scheduled_time"]).expect("scheduled_time");
    let staged_ledger_hash = &blockchain_state["staged_ledger_hash"];
    let non_snark = &staged_ledger_hash["non_snark"];
    let diffs = (&json["staged_ledger_diff"]["diff"]).as_array();
    let block_hash = Arc::new(block_hash);

    process_block_data(
        &pool,
        &block_hash,
        protocol_state,
        body,
        blockchain_state,
        scheduled_time,
        consensus_state,
        staged_ledger_hash,
        non_snark,
        height,
    )
    .await?;

    // Process after blocks since these objects link to blocks
    process_epoch_and_commands(&pool, &block_hash, consensus_state, diffs).await?;

    Ok(())
}

async fn process_block_data(
    pool: &DbPool,
    block_hash: &Arc<String>,
    protocol_state: &Value,
    body: &Value,
    blockchain_state: &Value,
    scheduled_time: i64,
    consensus_state: &Value,
    staged_ledger_hash: &Value,
    non_snark: &Value,
    height: i64,
) -> Result<(), edgedb_tokio::Error> {
    let query = format!(
        "with
        block := (
            insert Block {{
                hash := '{}',
                previous_hash := '{}',
                genesis_hash := '{}',
                blockchain_length := <int64>$0,
                epoch := <int64>$1,
                global_slot_since_genesis := <int64>$2,
                scheduled_time := <int64>$3,
                total_currency := <int64>$4,
                stake_winner := {},
                creator := {},
                coinbase_target := {},
                supercharge_coinbase := <bool>$5,
                has_ancestor_in_same_checkpoint_window := <bool>$6,
                min_window_density := <int64>$7,
                last_vrf_output := '{}'
            }}
        ),
        blockchain_state := (
            insert BlockchainState {{
                block := block,
                snarked_ledger_hash := '{}',
                genesis_ledger_hash := '{}',
                snarked_next_available_token := <int64>$8,
                timestamp := <int64>$9
            }}
        )
        insert StagedLedgerHash {{
            blockchain_state := blockchain_state,
            non_snark_ledger_hash := '{}',
            non_snark_aux_hash := '{}',
            non_snark_pending_coinbase_aux := '{}',
            pending_coinbase_hash := '{}'
        }};",
        block_hash.as_str(),
        protocol_state["previous_state_hash"]
            .as_str()
            .expect("previous_state_hash"),
        body["genesis_state_hash"]
            .as_str()
            .expect("genesis_state_hash"),
        account_link(&consensus_state["block_stake_winner"]),
        account_link(&consensus_state["block_creator"]),
        account_link(&consensus_state["coinbase_receiver"]),
        consensus_state["last_vrf_output"]
            .as_str()
            .expect("last_vrf_output"),
        blockchain_state["snarked_ledger_hash"]
            .as_str()
            .expect("snarked_ledger_hash"),
        blockchain_state["genesis_ledger_hash"]
            .as_str()
            .expect("genesis_ledger_hash"),
        non_snark["ledger_hash"].as_str().expect("ledger_hash"),
        non_snark["aux_hash"].as_str().expect("aux_hash"),
        non_snark["pending_coinbase_aux"]
            .as_str()
            .expect("pending_coinbase_aux"),
        staged_ledger_hash["pending_coinbase_hash"]
            .as_str()
            .expect("pending_coinbase_hash")
    )
    .to_string();

    // Clone/convert all numeric values needed in the closure
    let epoch_count = to_i64(&consensus_state["epoch_count"]);
    let global_slot = to_i64(&consensus_state["global_slot_since_genesis"]);

    let total_currency = to_i64(&consensus_state["total_currency"]);
    let supercharge_coinbase = consensus_state["supercharge_coinbase"].as_bool();
    let has_ancestor = consensus_state["has_ancestor_in_same_checkpoint_window"].as_bool();
    let min_window_density = to_i64(&consensus_state["min_window_density"]);
    let snarked_next_token = to_i64(&blockchain_state["snarked_next_available_token"]);
    let timestamp = to_i64(&blockchain_state["timestamp"]);

    pool.execute(
        query,
        (
            height,
            epoch_count,
            global_slot,
            scheduled_time,
            total_currency,
            supercharge_coinbase,
            has_ancestor,
            min_window_density,
            snarked_next_token,
            timestamp,
        ),
    )
    .await
}

async fn process_epoch_data(
    pool: &DbPool,
    epoch_type: &str,
    block_hash: &str,
    epoch_data: &Value,
    ledger: &Value,
) -> Result<(), edgedb_tokio::Error> {
    let query = format!(
        "insert {}EpochData {{
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
    .to_string();

    let ledger_hash = ledger["hash"].to_string();
    let total_currency = to_i64(&ledger["total_currency"]);
    let seed = epoch_data["seed"].to_string();
    let start_checkpoint = epoch_data["start_checkpoint"].to_string();
    let lock_checkpoint = epoch_data["lock_checkpoint"].to_string();
    let epoch_length = to_i64(&epoch_data["epoch_length"]);

    pool.execute(
        query,
        (
            ledger_hash,
            total_currency,
            seed,
            start_checkpoint,
            lock_checkpoint,
            epoch_length,
        ),
    )
    .await
}

async fn process_epoch_and_commands(
    pool: &DbPool,
    block_hash: &Arc<String>,
    consensus_state: &Value,
    diffs: Option<&Array>,
) -> Result<(), edgedb_tokio::Error> {
    for epoch_type in ["staking", "next"] {
        let epoch_data = &consensus_state[format!("{}_epoch_data", epoch_type).as_str()];
        let ledger = &epoch_data["ledger"];

        // Process epochs and commands concurrently
        let (epoch_future, commands_future) = futures::future::join(
            process_epoch_data(pool, epoch_type, block_hash, epoch_data, ledger),
            process_commands(pool, block_hash, diffs),
        )
        .await;

        epoch_future?;
        commands_future?;
    }

    Ok(())
}

async fn process_commands(
    pool: &DbPool,
    block_hash: &String,
    diffs: Option<&Array>,
) -> Result<(), edgedb_tokio::Error> {
    if let Some(diffs) = diffs {
        for diff in diffs {
            // Clone values needed for the closure
            let block_hash = block_hash.clone();
            let diff = diff.clone();

            // Process each command type in parallel
            let (_, _, _) = tokio::try_join!(
                snark_jobs(pool, &block_hash, &diff),
                user_commands(pool, &block_hash, &diff),
                internal_commands(pool, &block_hash, &diff)
            )?;
        }
    }
    Ok(())
}

async fn snark_jobs(
    pool: &DbPool,
    block_hash: &String,
    diff: &Value,
) -> Result<(), edgedb_tokio::Error> {
    if let Some(completed_works) = diff["completed_works"].as_array() {
        for job in completed_works {
            let query = format!(
                "insert SNARKJob {{
                    block := {},
                    prover := {},
                    fee := <decimal>$0
                }};",
                block_link(block_hash),
                account_link(&job["prover"])
            )
            .to_string();

            let fee = to_decimal(&job["fee"]);

            pool.execute(query, (fee,)).await?;
        }
    }
    Ok(())
}

async fn user_commands(
    pool: &DbPool,
    block_hash: &String,
    diff: &Value,
) -> Result<(), edgedb_tokio::Error> {
    if let Some(commands) = diff["commands"].as_array() {
        for command in commands {
            let data1 = &command["data"][1];
            let payload = &data1["payload"];
            let common = &payload["common"];
            let body1 = &payload["body"][1];
            let status = &command["status"];
            let status_1 = &status[1];
            let status_2 = &status[2];

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
                created_token := '{}'",
                block_link(block_hash),
                status[0].as_str().unwrap(),
                to_decimal(&status_2["source_balance"]).unwrap().to_string(),
                to_decimal(&status_2["receiver_balance"])
                    .unwrap_or_default()
                    .to_string(),
                to_decimal(&common["fee"]).unwrap_or_default().to_string(),
                account_link(&common["fee_payer_pk"]),
                to_decimal(&status_2["fee_payer_balance"])
                    .unwrap_or_default()
                    .to_string(),
                common["fee_token"].as_str().unwrap(),
                to_decimal(&status_1["fee_payer_account_creation_fee_paid"])
                    .unwrap_or_default()
                    .to_string(),
                to_decimal(&status_1["receiver_account_creation_fee_paid"])
                    .unwrap_or_default()
                    .to_string(),
                to_i64(&common["nonce"]).unwrap_or_default(),
                to_i64(&common["valid_until"]).unwrap_or_default(),
                common["memo"].as_str().unwrap_or_default(),
                account_link(&data1["signer"]),
                data1["signature"].as_str().unwrap(),
                status_1["created_token"].as_str().unwrap_or_default(),
            );

            match payload["body"][0].as_str().unwrap() {
                "Stake_delegation" => {
                    let delegation = &body1[1];
                    let query = format!(
                        "insert StakingDelegation {{
                            {},
                            source := {},
                            target := {},
                        }};",
                        command,
                        account_link(&delegation["delegator"]),
                        account_link(&delegation["new_delegate"])
                    )
                    .to_string();

                    pool.execute(query, ()).await?;
                }
                "Payment" => {
                    let query = format!(
                        "insert Payment {{
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
                    .to_string();

                    let amount = to_decimal(&body1["amount"]);
                    let token_id = to_i64(&body1["token_id"]);

                    pool.execute(query, (amount, token_id)).await?;
                }
                _ => {
                    println!("Unmatched {:?}", payload["body"][0].as_str().unwrap())
                }
            }
        }
    }
    Ok(())
}

async fn internal_commands(
    pool: &DbPool,
    block_hash: &String,
    diff: &Value,
) -> Result<(), edgedb_tokio::Error> {
    if let Some(balances) = diff["internal_command_balances"].as_array() {
        for internal_command in balances {
            let internal_command_1 = &internal_command[1];
            match internal_command[0].as_str().unwrap() {
                "Coinbase" => {
                    let query = format!(
                        "insert Coinbase {{
                            block := {},
                            target_balance := <decimal>$0
                        }};",
                        block_link(block_hash)
                    )
                    .to_string();

                    let target_balance =
                        to_decimal(&internal_command_1["coinbase_receiver_balance"]);

                    pool.execute(query, (target_balance,)).await?;
                }
                "Fee_transfer" => {
                    let query = format!(
                        "insert FeeTransfer {{
                            block := {},
                            target1_balance := <decimal>$0,
                            target2_balance := <optional decimal>$1
                        }};",
                        block_link(block_hash)
                    )
                    .to_string();

                    let target1_balance = to_decimal(&internal_command_1["receiver1_balance"]);
                    let target2_balance = to_decimal(&internal_command_1["receiver2_balance"]);

                    pool.execute(query, (target1_balance, target2_balance))
                        .await?;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn block_link(block_hash: &str) -> String {
    format!("(select Block filter .hash = '{block_hash}')")
}

const ACCOUNTS_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"B62.{52}$").expect("Failed to compile accounts regex"));

fn extract_accounts(value: &Value) -> HashSet<String> {
    let mut accounts = HashSet::new();

    match value.get_type() {
        JsonType::String => {
            if let Some(s) = value.as_str() {
                if ACCOUNTS_REGEX.is_match(s) {
                    accounts.insert(s.to_string());
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
