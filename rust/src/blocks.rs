use crate::{
    files::{extract_digits_from_file_name, extract_hash_from_file_name, get_file_paths},
    get_db_connection,
};
use anyhow::Result;
use tracing::{error, info};

pub fn run(blocks_dir: &str) -> Result<()> {
    let db = get_db_connection()?;

    // Create temporary table for raw JSON data
    db.execute_batch(
        "CREATE TEMPORARY TABLE raw_blocks (
            hash VARCHAR,
            height BIGINT,
            data JSON
        );",
    )?;

    let paths = get_file_paths(blocks_dir)?;

    for path in &paths {
        let file_path = path.to_str().unwrap();
        let block_hash = extract_hash_from_file_name(path);
        let height = extract_digits_from_file_name(path);

        info!("Processing file: {}", file_path);

        let sql = format!(
            "INSERT INTO raw_blocks
                SELECT '{}' as hash, {} as height, data
                FROM read_json('{}') as data;",
            block_hash, height, file_path
        );

        match db.execute_batch(&sql) {
            Ok(_) => info!("Successfully loaded block {}", block_hash),
            Err(e) => {
                error!("Error loading block {}: {}", block_hash, e);
            }
        }
    }

    // Create temp views to help with JSON extraction
    db.execute_batch(
        r#"
            -- Create views for easier data access
            CREATE TEMPORARY VIEW temp_block_diffs AS
            SELECT
                hash,
                json_extract(data, '$.staged_ledger_diff.diff') as diff
            FROM raw_blocks;

            -- Extract completed works (SNARK jobs)
            CREATE TEMPORARY VIEW temp_completed_works AS
            SELECT
                hash,
                json_extract(json_extract(diff, '$[0]'), '$.completed_works[*]') as work
            FROM temp_block_diffs
            WHERE json_extract(json_extract(diff, '$[0]'), '$.completed_works') IS NOT NULL;

            -- Extract user commands
            CREATE TEMPORARY VIEW temp_user_commands AS
            SELECT
                hash,
                json_extract(json_extract(diff, '$[0]'), '$.commands[*]') as cmd
            FROM temp_block_diffs
            WHERE json_extract(json_extract(diff, '$[0]'), '$.commands') IS NOT NULL;

            -- Extract internal commands
            CREATE TEMPORARY VIEW temp_internal_commands AS
            SELECT
                hash,
                json_extract(json_extract(diff, '$[0]'), '$.internal_command_balances[*]') as cmd
            FROM temp_block_diffs
            WHERE json_extract(json_extract(diff, '$[0]'), '$.internal_command_balances') IS NOT NULL;
            "#,
    )?;

    db.execute_batch(
        "

        -- Insert accounts
        INSERT INTO accounts (public_key)
        SELECT DISTINCT val FROM (
            SELECT json_extract_string(data, '$.protocol_state.body.consensus_state.stake_winner') as val FROM raw_blocks
            UNION ALL
            SELECT json_extract_string(data, '$.protocol_state.body.consensus_state.block_creator') FROM raw_blocks
            UNION ALL
            SELECT json_extract_string(data, '$.protocol_state.body.consensus_state.coinbase_receiver') FROM raw_blocks
            UNION ALL
            SELECT json_extract_string(work, '$.prover') FROM temp_completed_works
            UNION ALL
            SELECT json_extract_string(cmd, '$.data[1].payload.body[1].delegator') FROM temp_user_commands
            UNION ALL
            SELECT json_extract_string(cmd, '$.data[1].payload.body[1].source_pk') FROM temp_user_commands
            UNION ALL
            SELECT json_extract_string(cmd, '$.data[1].payload.body[1].new_delegate') FROM temp_user_commands
            UNION ALL
            SELECT json_extract_string(cmd, '$.data[1].payload.body[1].receiver_pk') FROM temp_user_commands
            UNION ALL
            SELECT json_extract_string(cmd, '$.data[1].payload.common.fee_payer_pk') FROM temp_user_commands
            UNION ALL
            SELECT json_extract_string(cmd, '$.data[1].signer') FROM temp_user_commands
        ) unique_keys
        WHERE val IS NOT NULL
        AND val SIMILAR TO 'B62[0-9A-Za-z]{52}'
        ON CONFLICT (public_key) DO NOTHING;",
    )?;

    db.execute_batch(
        "

             -- Insert blocks
             INSERT INTO blocks (
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
                 last_vrf_output,
                 min_window_density,
                 has_ancestor_in_same_checkpoint_window
             )
             SELECT
                 hash,
                 json_extract_string(data, '$.protocol_state.previous_state_hash'),
                 json_extract_string(data, '$.protocol_state.body.genesis_state_hash'),
                 height,
                 CAST(json_extract(data, '$.protocol_state.body.consensus_state.epoch_count') AS BIGINT),
                 CAST(json_extract(data, '$.protocol_state.body.consensus_state.global_slot_since_genesis') AS BIGINT),
                 CAST(json_extract(data, '$.scheduled_time') AS BIGINT),
                 CAST(json_extract(data, '$.protocol_state.body.consensus_state.total_currency') AS BIGINT),
                 json_extract_string(data, '$.protocol_state.body.consensus_state.block_stake_winner'),
                 json_extract_string(data, '$.protocol_state.body.consensus_state.block_creator'),
                 json_extract_string(data, '$.protocol_state.body.consensus_state.coinbase_receiver'),
                 CAST(json_extract(data, '$.protocol_state.body.consensus_state.supercharge_coinbase') AS BOOLEAN),
                 json_extract_string(data, '$.protocol_state.body.consensus_state.last_vrf_output'),
                 CAST(json_extract(data, '$.protocol_state.body.consensus_state.min_window_density') AS BIGINT),
                 CAST(json_extract(data, '$.protocol_state.body.consensus_state.has_ancestor_in_same_checkpoint_window') AS BOOLEAN)
             FROM raw_blocks;",
    )?;
    db.execute_batch(
        "

             -- Insert blockchain states
             INSERT INTO blockchain_states (
                 block_hash,
                 snarked_ledger_hash,
                 genesis_ledger_hash,
                 snarked_next_available_token,
                 timestamp
             )
             SELECT
                 hash,
                 json_extract_string(data, '$.protocol_state.body.blockchain_state.snarked_ledger_hash'),
                 json_extract_string(data, '$.protocol_state.body.blockchain_state.genesis_ledger_hash'),
                 CAST(json_extract(data, '$.protocol_state.body.blockchain_state.snarked_next_available_token') AS BIGINT),
                 CAST(json_extract(data, '$.protocol_state.body.blockchain_state.timestamp') AS BIGINT)
             FROM raw_blocks;

             -- Insert staged ledger hashes
             INSERT INTO staged_ledger_hashes (
                 block_hash,
                 non_snark_ledger_hash,
                 non_snark_aux_hash,
                 non_snark_pending_coinbase_aux,
                 pending_coinbase_hash
             )
             SELECT
                 hash,
                 json_extract_string(data, '$.protocol_state.body.blockchain_state.staged_ledger_hash.non_snark.ledger_hash'),
                 json_extract_string(data, '$.protocol_state.body.blockchain_state.staged_ledger_hash.non_snark.aux_hash'),
                 json_extract_string(data, '$.protocol_state.body.blockchain_state.staged_ledger_hash.non_snark.pending_coinbase_aux'),
                 json_extract_string(data, '$.protocol_state.body.blockchain_state.staged_ledger_hash.pending_coinbase_hash')
             FROM raw_blocks;
             ",
    )?;
    db.execute_batch(
        r#"
            INSERT INTO snark_jobs (id, block_hash, prover, fee)
            SELECT
                nextval('snark_jobs_id_seq'),
                hash,
                json_extract_string(work, '$.prover'),
                CAST(json_extract(work, '$.fee') AS DECIMAL)
            FROM temp_completed_works
            WHERE work IS NOT NULL;
            "#,
    )?;

    db.execute_batch(
        r#"
            INSERT INTO user_commands
            SELECT
                nextval('user_commands_id_seq'),
                hash,
                json_extract_string(cmd, '$.status[0]'),
                CASE json_extract_string(cmd, '$.data[1].payload.body[0]')
                    WHEN 'Stake_delegation' THEN json_extract_string(cmd, '$.data[1].payload.body[1].delegator')
                    WHEN 'Payment' THEN json_extract_string(cmd, '$.data[1].payload.body[1].source_pk')
                END,
                CAST(json_extract(cmd, '$.status[2].source_balance') AS DECIMAL),
                CASE json_extract_string(cmd, '$.data[1].payload.body[0]')
                    WHEN 'Stake_delegation' THEN json_extract_string(cmd, '$.data[1].payload.body[1].new_delegate')
                    WHEN 'Payment' THEN json_extract_string(cmd, '$.data[1].payload.body[1].receiver_pk')
                END,
                CAST(json_extract(cmd, '$.status[2].receiver_balance') AS DECIMAL),
                CAST(json_extract(cmd, '$.data[1].payload.common.fee') AS DECIMAL),
                json_extract_string(cmd, '$.data[1].payload.common.fee_payer_pk'),
                CAST(json_extract(cmd, '$.status[2].fee_payer_balance') AS DECIMAL),
                json_extract_string(cmd, '$.data[1].payload.common.fee_token'),
                CAST(json_extract(cmd, '$.status[1].fee_payer_account_creation_fee_paid') AS DECIMAL),
                CAST(json_extract(cmd, '$.status[1].receiver_account_creation_fee_paid') AS DECIMAL),
                CAST(json_extract(cmd, '$.data[1].payload.common.nonce') AS BIGINT),
                CAST(json_extract(cmd, '$.data[1].payload.common.valid_until') AS BIGINT),
                json_extract_string(cmd, '$.data[1].payload.common.memo'),
                json_extract_string(cmd, '$.data[1].signer'),
                json_extract_string(cmd, '$.data[1].signature'),
                json_extract(cmd, '$.status[1].created_token'),
                CASE json_extract_string(cmd, '$.data[1].payload.body[0]')
                    WHEN 'Stake_delegation' THEN 'staking_delegation'
                    WHEN 'Payment' THEN 'payment'
                END,
                CAST(json_extract(cmd, '$.data[1].payload.body[1].token_id') AS BIGINT),
                CAST(json_extract(cmd, '$.data[1].payload.body[1].amount') AS DECIMAL)
            FROM temp_user_commands
            WHERE json_extract_string(cmd, '$.data[1].payload.body[0]') IN ('Stake_delegation', 'Payment');
            "#,
    )?;

    db.execute_batch(
        r#"
            INSERT INTO internal_commands
            SELECT
                nextval('internal_commands_id_seq'),
                hash,
                CASE json_extract_string(cmd, '$[0]')
                    WHEN 'Coinbase' THEN 'coinbase'
                    WHEN 'Fee_transfer' THEN 'fee_transfer'
                END,
                CAST(json_extract(cmd, '$[1].coinbase_receiver_balance') AS DECIMAL),
                CAST(json_extract(cmd, '$[1].receiver2_balance') AS DECIMAL)
            FROM temp_internal_commands
            WHERE cmd IS NOT NULL;
            "#,
    )?;

    db.execute_batch(
        "
             -- Insert epoch data
             WITH epoch_types(path, label) AS (
                 VALUES
                     ('staking_epoch_data', 'staking'),
                     ('next_epoch_data', 'next')
             ),
             epoch_data_source AS (
                 SELECT
                     r.hash,
                     json_extract_string(r.data, '$.protocol_state.body.consensus_state.' || e.path || '.ledger.hash') as ledger_hash,
                     CAST(json_extract(r.data, '$.protocol_state.body.consensus_state.' || e.path || '.ledger.total_currency') AS BIGINT) as total_currency,
                     json_extract_string(r.data, '$.protocol_state.body.consensus_state.' || e.path || '.seed') as seed,
                     json_extract_string(r.data, '$.protocol_state.body.consensus_state.' || e.path || '.start_checkpoint') as start_checkpoint,
                     json_extract_string(r.data, '$.protocol_state.body.consensus_state.' || e.path || '.lock_checkpoint') as lock_checkpoint,
                     CAST(json_extract(r.data, '$.protocol_state.body.consensus_state.' || e.path || '.epoch_length') AS BIGINT) as epoch_length,
                     e.label as type
                 FROM raw_blocks r
                 CROSS JOIN epoch_types e
             )
             INSERT INTO epoch_data (
                 block_hash,
                 ledger_hash,
                 total_currency,
                 seed,
                 start_checkpoint,
                 lock_checkpoint,
                 epoch_length,
                 type
             )
             SELECT * FROM epoch_data_source;",
    )?;

    Ok(())
}
