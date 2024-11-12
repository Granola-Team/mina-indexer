INSERT INTO internal_commands (
    id,
    block_hash,
    data_type,
    target1_balance,
    target2_balance
)
SELECT
    nextval('internal_commands_id_seq'),
    hash,
    CASE data_type
        WHEN 'Coinbase' THEN 'coinbase'
        WHEN 'Fee_transfer' THEN 'fee_transfer'
    END,
    CASE data_type
        WHEN
            'Coinbase'
            THEN
                cast(
                    json_extract_string(
                        balances, '$[1].coinbase_receiver_balance'
                    ) AS DECIMAL(38, 9)
                )
        WHEN
            'Fee_transfer'
            THEN
                cast(
                    json_extract_string(
                        balances, '$[1].receiver1_balance'
                    ) AS DECIMAL(38, 9)
                )
    END AS target1_balance,
    CASE data_type
        WHEN
            'Coinbase'
            THEN
                cast(
                    nullif(
                        json_extract_string(
                            balances, '$[1].fee_transfer_receiver_balance'
                        ),
                        'null'
                    ) AS DECIMAL(38, 9)
                )
        WHEN
            'Fee_transfer'
            THEN
                cast(
                    nullif(
                        json_extract_string(balances, '$[1].receiver2_balance'),
                        'null'
                    ) AS DECIMAL(38, 9)
                )
    END AS target2_balance
FROM temp_internal_commands
WHERE balances IS NOT NULL;
