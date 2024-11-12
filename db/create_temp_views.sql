-- Create base views for common JSON paths
CREATE TEMPORARY VIEW json_paths AS
SELECT
    '$.protocol_state.body' AS protocol_body,
    '$.protocol_state.body.consensus_state' AS consensus_state,
    '$.protocol_state.body.blockchain_state' AS blockchain_state,
    '$.staged_ledger_diff.diff[0]' AS staged_ledger,
    '$.protocol_state.previous_state_hash' AS previous_hash,
    '$.scheduled_time' AS scheduled_time;

-- Create main extracted state view
CREATE TEMPORARY VIEW extracted_state AS
SELECT
    hash,
    height,
    json,
    cast(json_extract(json, scheduled_time) AS BIGINT) AS scheduled_time,
    json_extract(json, protocol_body) AS body,
    json_extract(json, consensus_state) AS consensus,
    json_extract(json, blockchain_state) AS blockchain,
    json_extract(json, staged_ledger) AS staged_ledger,
    json_extract_string(json, previous_hash) AS previous_hash
FROM raw_blocks
CROSS JOIN json_paths;

-- Create completed works view
CREATE TEMPORARY VIEW temp_completed_works AS
SELECT
    hash,
    json_extract(staged_ledger, '$.completed_works') AS works
FROM extracted_state
WHERE json_extract(staged_ledger, '$.completed_works') IS NOT NULL;

-- Create user commands view
CREATE TEMPORARY VIEW temp_user_commands AS
SELECT
    hash,
    command AS full_command,
    json_extract_string(command, '$.data[0]') AS data_type,
    json_extract(command, '$.data[1]') AS cmd_data,
    json_extract(json_extract(command, '$.data[1]'), '$.payload') AS payload,
    -- Common fields
    json_extract_string(
        json_extract(command, '$.data[1]'), '$.payload.common.fee'
    ) AS fee,
    json_extract_string(
        json_extract(command, '$.data[1]'), '$.payload.common.fee_token'
    ) AS fee_token,
    json_extract_string(
        json_extract(command, '$.data[1]'), '$.payload.common.fee_payer_pk'
    ) AS fee_payer_pk,
    json_extract_string(
        json_extract(command, '$.data[1]'), '$.payload.common.nonce'
    ) AS nonce,
    json_extract_string(
        json_extract(command, '$.data[1]'), '$.payload.common.valid_until'
    ) AS valid_until,
    json_extract_string(
        json_extract(command, '$.data[1]'), '$.payload.common.memo'
    ) AS memo,
    -- Body fields
    json_extract_string(
        json_extract(command, '$.data[1]'), '$.payload.body[0]'
    ) AS command_type,
    json_extract_string(
        json_extract(command, '$.data[1]'), '$.payload.body[1].source_pk'
    ) AS source_pk,
    json_extract_string(
        json_extract(command, '$.data[1]'), '$.payload.body[1].receiver_pk'
    ) AS receiver_pk,
    json_extract_string(
        json_extract(command, '$.data[1]'), '$.payload.body[1].amount'
    ) AS amount,
    json_extract_string(
        json_extract(command, '$.data[1]'), '$.payload.body[1].token_id'
    ) AS token_id,
    -- Delegation fields
    json_extract_string(
        json_extract(command, '$.data[1]'), '$.payload.body[1].delegator'
    ) AS delegator,
    json_extract_string(
        json_extract(command, '$.data[1]'), '$.payload.body[1].new_delegate'
    ) AS new_delegate,
    -- Signature fields
    json_extract_string(json_extract(command, '$.data[1]'), '$.signer')
        AS signer,
    json_extract_string(json_extract(command, '$.data[1]'), '$.signature')
        AS signature,
    -- Status fields
    json_extract_string(command, '$.status[0]') AS status,
    json_extract_string(
        command, '$.status[1].fee_payer_account_creation_fee_paid'
    ) AS fee_payer_account_creation_fee_paid,
    json_extract_string(
        command, '$.status[1].receiver_account_creation_fee_paid'
    ) AS receiver_account_creation_fee_paid,
    json_extract_string(command, '$.status[1].created_token') AS created_token,
    json_extract_string(command, '$.status[2].fee_payer_balance')
        AS fee_payer_balance,
    json_extract_string(command, '$.status[2].source_balance')
        AS source_balance,
    json_extract_string(command, '$.status[2].receiver_balance')
        AS receiver_balance
FROM (
    SELECT
        hash,
        json_extract(staged_ledger, '$.commands[0]') AS command
    FROM extracted_state
    WHERE json_extract(staged_ledger, '$.commands[0]') IS NOT NULL
);

-- Create internal commands view
CREATE TEMPORARY VIEW temp_internal_commands AS
SELECT
    hash,
    json_extract_string(staged_ledger, '$.internal_command_balances[0][0]')
        AS data_type,
    json_extract(staged_ledger, '$.internal_command_balances[0]') AS balances
FROM extracted_state
WHERE json_extract(staged_ledger, '$.internal_command_balances[0]') IS NOT NULL;
