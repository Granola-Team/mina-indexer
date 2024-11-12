-- Create base temporary table for raw JSON data
CREATE TEMPORARY TABLE raw_blocks (
    hash VARCHAR,
    height BIGINT,
    data JSON
);

-- Create base views for common JSON paths
CREATE TEMPORARY VIEW json_paths AS
SELECT
    '$.protocol_state.body' as protocol_body,
    '$.protocol_state.body.consensus_state' as consensus_state,
    '$.protocol_state.body.blockchain_state' as blockchain_state,
    '$.staged_ledger_diff.diff[0]' as staged_ledger,
    '$.protocol_state.previous_state_hash' as previous_hash,
    '$.scheduled_time' as scheduled_time;

-- Create main extracted state view
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

-- Create completed works view
CREATE TEMPORARY VIEW temp_completed_works AS
SELECT
    hash,
    json_extract(staged_ledger, '$.completed_works') as data
FROM extracted_state
WHERE json_extract(staged_ledger, '$.completed_works') IS NOT NULL;

-- Create user commands view
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

-- Create internal commands view
CREATE TEMPORARY VIEW temp_internal_commands AS
SELECT
    hash,
    json_extract_string(staged_ledger, '$.internal_command_balances[0][0]') as type,
    json_extract(staged_ledger, '$.internal_command_balances[0]') as data
FROM extracted_state
WHERE json_extract(staged_ledger, '$.internal_command_balances[0]') IS NOT NULL;
