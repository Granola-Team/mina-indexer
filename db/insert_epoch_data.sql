WITH epoch_types (path, label) AS (
    VALUES
    ('staking_epoch_data', 'staking'),
    ('next_epoch_data', 'next')
),

epoch_data_source AS (
    SELECT
        hash AS block_hash,
        json_extract_string(consensus, '$.' || path || '.ledger.hash') AS ledger_hash,
        CAST(json_extract(consensus, '$.' || path || '.ledger.total_currency') AS BIGINT) AS total_currency,
        json_extract_string(consensus, '$.' || path || '.seed') AS seed,
        json_extract_string(consensus, '$.' || path || '.start_checkpoint') AS start_checkpoint,
        json_extract_string(consensus, '$.' || path || '.lock_checkpoint') AS lock_checkpoint,
        CAST(json_extract(consensus, '$.' || path || '.epoch_length') AS BIGINT) AS epoch_length,
        label AS data_type
    FROM extracted_state
    CROSS JOIN epoch_types
)

INSERT INTO epoch_data (
    block_hash,
    ledger_hash,
    total_currency,
    seed,
    start_checkpoint,
    lock_checkpoint,
    epoch_length,
    data_type
)
SELECT
    block_hash,
    ledger_hash,
    total_currency,
    seed,
    start_checkpoint,
    lock_checkpoint,
    epoch_length,
    data_type
FROM epoch_data_source;
