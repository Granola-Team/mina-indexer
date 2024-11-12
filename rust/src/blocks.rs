use crate::{
    files::{extract_digits_from_file_name, extract_hash_from_file_name, get_file_paths},
    get_db_connection,
};
use anyhow::Result;
use tracing::{error, info};

pub fn run(blocks_dir: &str) -> Result<()> {
    let paths = get_file_paths(blocks_dir)?;
    const BATCH_SIZE: usize = 5_000;

    for chunk in paths.chunks(BATCH_SIZE) {
        let mut db = get_db_connection()?;
        let tx = db.transaction()?;

        // Create temporary table for raw JSON data
        tx.execute_batch(
            "CREATE TEMPORARY TABLE raw_blocks (
                hash VARCHAR,
                height BIGINT,
                data JSON
            );",
        )?;

        // Process current batch of files
        for path in chunk {
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

            match tx.execute_batch(&sql) {
                Ok(_) => info!("Successfully loaded block {}", block_hash),
                Err(e) => error!("Error loading block {}: {}", block_hash, e),
            }
        }

        // Create base views for common JSON paths
        tx.execute_batch(
            r#"
            CREATE TEMPORARY VIEW json_paths AS
            SELECT
                '$.protocol_state.body' as protocol_body,
                '$.protocol_state.body.consensus_state' as consensus_state,
                '$.protocol_state.body.blockchain_state' as blockchain_state,
                '$.staged_ledger_diff.diff[0]' as staged_ledger,
                '$.protocol_state.previous_state_hash' as previous_hash,
                '$.scheduled_time' as scheduled_time;

                CREATE TEMPORARY VIEW extracted_state AS
                SELECT
                    hash,
                    height,
                    data,
                    json_extract(data, protocol_body) as body,
                    json_extract(data, consensus_state) as consensus,
                    json_extract(data, blockchain_state) as blockchain,
                    json_extract(data, staged_ledger) as staged_ledger,
                    json_extract_string(data, previous_hash) as previous_hash,
                    CAST(json_extract(data, scheduled_time) AS BIGINT) as scheduled_time
                FROM raw_blocks
                CROSS JOIN json_paths;

            CREATE TEMPORARY VIEW temp_completed_works AS
            SELECT
                hash,
                json_extract(staged_ledger, '$.completed_works') as data
            FROM extracted_state
            WHERE json_extract(staged_ledger, '$.completed_works') IS NOT NULL;

            CREATE TEMPORARY VIEW temp_user_commands AS
            SELECT
                hash,
                command as full_command,
                json_extract_string(command, '$.data[0]') as type,
                json_extract(command, '$.data[1]') as cmd_data,
                json_extract(json_extract(command, '$.data[1]'), '$.payload') as payload,
                -- Common fields
                json_extract_string(json_extract(command, '$.data[1]'), '$.payload.common.fee') as fee,
                json_extract_string(json_extract(command, '$.data[1]'), '$.payload.common.fee_token') as fee_token,
                json_extract_string(json_extract(command, '$.data[1]'), '$.payload.common.fee_payer_pk') as fee_payer_pk,
                json_extract_string(json_extract(command, '$.data[1]'), '$.payload.common.nonce') as nonce,
                json_extract_string(json_extract(command, '$.data[1]'), '$.payload.common.valid_until') as valid_until,
                json_extract_string(json_extract(command, '$.data[1]'), '$.payload.common.memo') as memo,
                -- Body fields
                json_extract_string(json_extract(command, '$.data[1]'), '$.payload.body[0]') as command_type,
                json_extract_string(json_extract(command, '$.data[1]'), '$.payload.body[1].source_pk') as source_pk,
                json_extract_string(json_extract(command, '$.data[1]'), '$.payload.body[1].receiver_pk') as receiver_pk,
                json_extract_string(json_extract(command, '$.data[1]'), '$.payload.body[1].amount') as amount,
                json_extract_string(json_extract(command, '$.data[1]'), '$.payload.body[1].token_id') as token_id,
                -- Delegation fields
                json_extract_string(json_extract(command, '$.data[1]'), '$.payload.body[1].delegator') as delegator,
                json_extract_string(json_extract(command, '$.data[1]'), '$.payload.body[1].new_delegate') as new_delegate,
                -- Signature fields
                json_extract_string(json_extract(command, '$.data[1]'), '$.signer') as signer,
                json_extract_string(json_extract(command, '$.data[1]'), '$.signature') as signature,
                -- Status fields
                json_extract_string(command, '$.status[0]') as status,
                json_extract_string(command, '$.status[1].fee_payer_account_creation_fee_paid') as fee_payer_account_creation_fee_paid,
                json_extract_string(command, '$.status[1].receiver_account_creation_fee_paid') as receiver_account_creation_fee_paid,
                json_extract_string(command, '$.status[1].created_token') as created_token,
                json_extract_string(command, '$.status[2].fee_payer_balance') as fee_payer_balance,
                json_extract_string(command, '$.status[2].source_balance') as source_balance,
                json_extract_string(command, '$.status[2].receiver_balance') as receiver_balance
            FROM (
                SELECT
                    hash,
                    json_extract(staged_ledger, '$.commands[0]') as command
                FROM extracted_state
                WHERE json_extract(staged_ledger, '$.commands[0]') IS NOT NULL
            );

            CREATE TEMPORARY VIEW temp_internal_commands AS
            SELECT
                hash,
                json_extract_string(staged_ledger, '$.internal_command_balances[0][0]') as type,
                json_extract(staged_ledger, '$.internal_command_balances[0]') as data
            FROM extracted_state
            WHERE json_extract(staged_ledger, '$.internal_command_balances[0]') IS NOT NULL;
            "#,
        )?;

        // Insert data into permanent tables
        tx.execute_batch(
            r#"
            -- Insert accounts
            INSERT INTO accounts (public_key)
            SELECT DISTINCT val FROM (
                -- Consensus state related accounts
                SELECT json_extract_string(consensus, '$.block_stake_winner') as val FROM extracted_state
                UNION ALL
                SELECT json_extract_string(consensus, '$.block_creator') FROM extracted_state
                UNION ALL
                SELECT json_extract_string(consensus, '$.coinbase_receiver') FROM extracted_state
                -- Snark work related accounts
                UNION ALL
                SELECT json_extract_string(data, '$.prover') FROM temp_completed_works
                -- User command related accounts
                UNION ALL
                SELECT delegator FROM temp_user_commands WHERE delegator IS NOT NULL
                UNION ALL
                SELECT source_pk FROM temp_user_commands WHERE source_pk IS NOT NULL
                UNION ALL
                SELECT new_delegate FROM temp_user_commands WHERE new_delegate IS NOT NULL
                UNION ALL
                SELECT receiver_pk FROM temp_user_commands WHERE receiver_pk IS NOT NULL
                UNION ALL
                SELECT fee_payer_pk FROM temp_user_commands WHERE fee_payer_pk IS NOT NULL
                UNION ALL
                SELECT signer FROM temp_user_commands WHERE signer IS NOT NULL
                -- Internal commands related accounts
                UNION ALL
                SELECT json_extract_string(data, '$[1].receiver') FROM temp_internal_commands
                UNION ALL
                SELECT json_extract_string(data, '$[1].fee_transfer_receiver') FROM temp_internal_commands
            ) unique_keys
            WHERE val IS NOT NULL
            AND val SIMILAR TO 'B62[0-9A-Za-z]{52}'
            ON CONFLICT (public_key) DO NOTHING;
            "#,
        )?;

        tx.execute_batch(
            r#"

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
                previous_hash,
                json_extract_string(body, '$.genesis_state_hash'),
                height,
                CAST(json_extract(consensus, '$.epoch_count') AS BIGINT),
                CAST(json_extract(consensus, '$.global_slot_since_genesis') AS BIGINT),
                scheduled_time,
                CAST(json_extract(consensus, '$.total_currency') AS BIGINT),
                json_extract_string(consensus, '$.block_stake_winner'),
                json_extract_string(consensus, '$.block_creator'),
                json_extract_string(consensus, '$.coinbase_receiver'),
                CAST(json_extract(consensus, '$.supercharge_coinbase') AS BOOLEAN),
                json_extract_string(consensus, '$.last_vrf_output'),
                CAST(json_extract(consensus, '$.min_window_density') AS BIGINT),
                CAST(json_extract(consensus, '$.has_ancestor_in_same_checkpoint_window') AS BOOLEAN)
            FROM extracted_state;
            "#,
        )?;

        tx.execute_batch(
            r#"

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
                json_extract_string(blockchain, '$.snarked_ledger_hash'),
                json_extract_string(blockchain, '$.genesis_ledger_hash'),
                CAST(json_extract(blockchain, '$.snarked_next_available_token') AS BIGINT),
                CAST(json_extract(blockchain, '$.timestamp') AS BIGINT)
            FROM extracted_state;

            "#,
        )?;

        tx.execute_batch(
            r#"

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
                json_extract_string(blockchain, '$.staged_ledger_hash.non_snark.ledger_hash'),
                json_extract_string(blockchain, '$.staged_ledger_hash.non_snark.aux_hash'),
                json_extract_string(blockchain, '$.staged_ledger_hash.non_snark.pending_coinbase_aux'),
                json_extract_string(blockchain, '$.staged_ledger_hash.pending_coinbase_hash')
            FROM extracted_state;

            "#,
        )?;

        tx.execute_batch(
            r#"
            -- Insert snark jobs
            INSERT INTO snark_jobs (id, block_hash, prover, fee)
            SELECT
                nextval('snark_jobs_id_seq'),
                hash,
                json_extract_string(data, '$.prover'),
                CAST(json_extract(data, '$.fee') AS DECIMAL(38, 9))
            FROM temp_completed_works
            WHERE data IS NOT NULL;

            "#,
        )?;

        tx.execute_batch(
            r#"
            -- Insert user commands
            INSERT INTO user_commands
            SELECT
                nextval('user_commands_id_seq'),
                hash,
                CASE status
                    WHEN 'Applied' THEN 'applied'
                    WHEN 'Failed' THEN 'failed'
                END,
                CASE command_type
                    WHEN 'Stake_delegation' THEN delegator
                    WHEN 'Payment' THEN source_pk
                END,
                CAST(NULLIF(source_balance, 'null') AS DECIMAL(38, 9)),
                CASE command_type
                    WHEN 'Stake_delegation' THEN new_delegate
                    WHEN 'Payment' THEN receiver_pk
                END,
                CAST(NULLIF(receiver_balance, 'null') AS DECIMAL(38, 9)),
                CAST(fee AS DECIMAL(38, 9)),
                fee_payer_pk,
                CAST(NULLIF(fee_payer_balance, 'null') AS DECIMAL(38, 9)),
                fee_token,
                CAST(NULLIF(fee_payer_account_creation_fee_paid, 'null') AS DECIMAL(38, 9)),
                CAST(NULLIF(receiver_account_creation_fee_paid, 'null') AS DECIMAL(38, 9)),
                CAST(nonce AS BIGINT),
                CAST(valid_until AS BIGINT),
                memo,
                signer,
                signature,
                created_token,
                CASE command_type
                    WHEN 'Stake_delegation' THEN 'staking_delegation'
                    WHEN 'Payment' THEN 'payment'
                END,
                CAST(NULLIF(token_id, 'null') AS BIGINT),
                CAST(NULLIF(amount, 'null') AS DECIMAL(38, 9))
            FROM temp_user_commands
            WHERE command_type IN ('Stake_delegation', 'Payment');

            "#,
        )?;

        tx.execute_batch(
            r#"
            -- Insert internal commands
            INSERT INTO internal_commands (
                id,
                block_hash,
                type,
                target1_balance,
                target2_balance
            )
            SELECT
                nextval('internal_commands_id_seq'),
                hash,
                CASE type
                    WHEN 'Coinbase' THEN 'coinbase'
                    WHEN 'Fee_transfer' THEN 'fee_transfer'
                END,
                -- First target balance based on command type
                CASE type
                    WHEN 'Coinbase' THEN CAST(json_extract_string(data, '$[1].coinbase_receiver_balance') AS DECIMAL(38, 9))
                    WHEN 'Fee_transfer' THEN CAST(json_extract_string(data, '$[1].receiver1_balance') AS DECIMAL(38, 9))
                END as target1_balance,
                -- Second target balance based on command type
                CASE type
                    WHEN 'Coinbase' THEN CAST(NULLIF(json_extract_string(data, '$[1].fee_transfer_receiver_balance'), 'null') AS DECIMAL(38, 9))
                    WHEN 'Fee_transfer' THEN CAST(NULLIF(json_extract_string(data, '$[1].receiver2_balance'), 'null') AS DECIMAL(38, 9))
                END as target2_balance
            FROM temp_internal_commands
            WHERE data IS NOT NULL;

            "#,
        )?;

        tx.execute_batch(
            r#"
            -- Insert epoch data
            WITH epoch_types(path, label) AS (
                VALUES
                    ('staking_epoch_data', 'staking'),
                    ('next_epoch_data', 'next')
            ),
            epoch_data_source AS (
                SELECT
                    e.hash,
                    json_extract_string(e.consensus, '$.' || t.path || '.ledger.hash') as ledger_hash,
                    CAST(json_extract(e.consensus, '$.' || t.path || '.ledger.total_currency') AS BIGINT) as total_currency,
                    json_extract_string(e.consensus, '$.' || t.path || '.seed') as seed,
                    json_extract_string(e.consensus, '$.' || t.path || '.start_checkpoint') as start_checkpoint,
                    json_extract_string(e.consensus, '$.' || t.path || '.lock_checkpoint') as lock_checkpoint,
                    CAST(json_extract(e.consensus, '$.' || t.path || '.epoch_length') AS BIGINT) as epoch_length,
                    t.label as type
                FROM extracted_state e
                CROSS JOIN epoch_types t
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
            SELECT * FROM epoch_data_source;
            "#,
        )?;
    }

    Ok(())
}
