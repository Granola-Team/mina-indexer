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
    cast(json_extract(blockchain, '$.snarked_next_available_token') AS BIGINT),
    cast(json_extract(blockchain, '$.timestamp') AS BIGINT)
FROM extracted_state;
