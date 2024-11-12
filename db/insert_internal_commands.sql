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
    CASE type
        WHEN 'Coinbase' THEN CAST(json_extract_string(data, '$[1].coinbase_receiver_balance') AS DECIMAL(38, 9))
        WHEN 'Fee_transfer' THEN CAST(json_extract_string(data, '$[1].receiver1_balance') AS DECIMAL(38, 9))
    END as target1_balance,
    CASE type
        WHEN 'Coinbase' THEN CAST(NULLIF(json_extract_string(data, '$[1].fee_transfer_receiver_balance'), 'null') AS DECIMAL(38, 9))
        WHEN 'Fee_transfer' THEN CAST(NULLIF(json_extract_string(data, '$[1].receiver2_balance'), 'null') AS DECIMAL(38, 9))
    END as target2_balance
FROM temp_internal_commands
WHERE data IS NOT NULL;
