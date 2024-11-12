INSERT INTO accounts (public_key)
SELECT DISTINCT val FROM (
    -- Consensus state related accounts
    SELECT json_extract_string(consensus, '$.block_stake_winner') AS val FROM extracted_state
    UNION ALL
    SELECT json_extract_string(consensus, '$.block_creator') FROM extracted_state
    UNION ALL
    SELECT json_extract_string(consensus, '$.coinbase_receiver') FROM extracted_state
    -- SNARK work related accounts
    UNION ALL
    SELECT json_extract_string(works, '$.prover') FROM temp_completed_works
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
    SELECT json_extract_string(balances, '$[1].receiver') FROM temp_internal_commands
    UNION ALL
    SELECT json_extract_string(balances, '$[1].fee_transfer_receiver') FROM temp_internal_commands
)
WHERE val IS NOT NULL AND val SIMILAR TO 'B62[0-9A-Za-z]{52}' ON CONFLICT (public_key) DO NOTHING;
