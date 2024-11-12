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
